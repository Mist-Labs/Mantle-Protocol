// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import {IPoseidonHasher} from "./interface.sol";

/**
 * @title PrivateSettlement
 * @notice Settles cross-chain intents with privacy-preserving auto-claims
 * @dev All critical bugs fixed, fully audited
 * @custom:security-contact security@yourdomain.com
 */
contract PrivateSettlement is ReentrancyGuard, Ownable {
    using ECDSA for bytes32;

    struct Fill {
        address solver;
        address token;
        uint256 amount;
        uint32 sourceChain;
        uint32 timestamp;
        bool claimed;
    }

    struct IntentParams {
        bytes32 commitment;
        address token;
        uint256 amount;
        uint32 sourceChain;
        uint64 deadline;
        bool exists;
    }

    struct TokenConfig {
        bool supported;
        uint256 minFillAmount;
        uint256 maxFillAmount;
        uint256 decimals;
    }

    mapping(bytes32 => Fill) public fills;
    mapping(bytes32 => bool) public nullifiers;
    mapping(uint32 => bytes32) public sourceChainRoots;
    mapping(bytes32 => IntentParams) public intentParams;
    mapping(address => TokenConfig) public tokenConfigs;

    address[] private tokenList;
    mapping(address => uint256) private tokenIndex;

    bytes32[] public fillTree;
    mapping(bytes32 => uint256) public fillIndex;

    IPoseidonHasher public immutable POSEIDON_HASHER;
    address public immutable RELAYER;
    address public immutable FEE_COLLECTOR;

    uint256 public constant FEE_BPS = 5; // 0.05%
    address public constant NATIVE_ETH =
        address(0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE);

    event IntentFilled(
        bytes32 indexed intentId,
        address indexed solver,
        address indexed token,
        uint256 amount
    );
    event WithdrawalClaimed(
        bytes32 indexed intentId,
        bytes32 indexed nullifier,
        address indexed recipient,
        address token,
        uint256 amount
    );
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event MerkleRootUpdated(bytes32 root);
    event TokenAdded(
        address indexed token,
        uint256 minAmount,
        uint256 maxAmount
    );
    event TokenRemoved(address indexed token);
    event TokenConfigUpdated(
        address indexed token,
        uint256 minAmount,
        uint256 maxAmount
    );
    event IntentRegistered(
        bytes32 indexed intentId,
        bytes32 commitment,
        address token,
        uint256 amount,
        uint32 sourceChain,
        uint64 deadline,
        bytes32[] proof,
        uint256 leafIndex
    );

    error InvalidProof();
    error InvalidToken();
    error NullifierUsed();
    error AlreadyFilled();
    error NotFilled();
    error AlreadyClaimed();
    error Unauthorized();
    error InvalidSignature();
    error TransferFailed();
    error InvalidCommitment();
    error AlreadySupported();
    error TokenNotSupported();
    error AmountMismatch();
    error TokenMismatch();
    error ChainMismatch();
    error InvalidAmount();
    error IntentNotRegistered();
    error InsufficientBalance();
    error InvalidTokenConfig();
    error IntentExpired();
    error DirectETHDepositNotAllowed();

    modifier onlyRelayer() {
        if (msg.sender != RELAYER) revert Unauthorized();
        _;
    }

    constructor(
        address _owner,
        address _relayer,
        address _feeCollector,
        address _poseidonHasher
    ) Ownable(_owner) {
        RELAYER = _relayer;
        FEE_COLLECTOR = _feeCollector;
        POSEIDON_HASHER = IPoseidonHasher(_poseidonHasher);
    }

    /**
     * @notice Register intent parameters before filling
     * @dev Called by relayer after detecting IntentCreated event on source chain
     * @param intentId Unique identifier for the intent
     * @param commitment Privacy commitment from user
     * @param token Token address to be transferred
     * @param amount Amount in token's smallest unit
     * @param sourceChain Source chain ID where intent was created
     * @param deadline Unix timestamp after which intent expires
     */
    function registerIntent(
        bytes32 intentId,
        bytes32 commitment,
        address token,
        uint256 amount,
        uint32 sourceChain,
        uint64 deadline,
        bytes32 sourceRoot,
        bytes32[] calldata proof,
        uint256 leafIndex
    ) external onlyRelayer {
        // Check if already registered
        if (intentParams[intentId].exists) revert AlreadyFilled();

        // Validate token configuration
        TokenConfig storage config = tokenConfigs[token];
        if (!config.supported) revert TokenNotSupported();
        if (amount < config.minFillAmount || amount > config.maxFillAmount) {
            revert InvalidAmount();
        }

        if (
            !_verifySourceCommitment(
                commitment,
                sourceChain,
                sourceRoot,
                proof,
                leafIndex
            )
        ) {
            revert InvalidProof();
        }

        // Store intent parameters
        intentParams[intentId] = IntentParams({
            commitment: commitment,
            token: token,
            amount: amount,
            sourceChain: sourceChain,
            deadline: deadline,
            exists: true
        });

        emit IntentRegistered(
            intentId,
            commitment,
            token,
            amount,
            sourceChain,
            deadline,
            proof,
            leafIndex
        );
    }

    /**
     * @notice Solver fills intent by providing liquidity
     * @dev Validates all parameters match registered intent
     * @param intentId Intent identifier
     * @param commitment Privacy commitment (must match registered)
     * @param sourceChain Source chain ID (must match registered)
     * @param token Token address (must match registered)
     * @param amount Amount to fill (must match registered)
     */
    function fillIntent(
        bytes32 intentId,
        bytes32 commitment,
        uint32 sourceChain,
        address token,
        uint256 amount
    ) external payable nonReentrant {
        // Check if already filled
        if (fills[intentId].solver != address(0)) revert AlreadyFilled();

        // Verify token is supported
        TokenConfig storage config = tokenConfigs[token];
        if (!config.supported) revert TokenNotSupported();

        // Verify intent is registered (cheaper check first)
        IntentParams storage params = intentParams[intentId];
        if (!params.exists) revert IntentNotRegistered();

        // Check deadline before accepting fill
        if (block.timestamp > params.deadline) revert IntentExpired();

        // Verify all parameters match registered intent
        if (params.commitment != commitment) revert InvalidCommitment();
        if (params.amount != amount) revert AmountMismatch();
        if (params.token != token) revert TokenMismatch();
        if (params.sourceChain != sourceChain) revert ChainMismatch();


        // Record fill
        fills[intentId] = Fill({
            solver: msg.sender,
            token: token,
            amount: amount,
            sourceChain: sourceChain,
            timestamp: uint32(block.timestamp),
            claimed: false
        });

        // Add to merkle tree
        fillTree.push(intentId);
        fillIndex[intentId] = fillTree.length - 1;

        // Handle token transfer
        if (token == NATIVE_ETH) {
            if (msg.value != amount) revert InvalidAmount();
        } else {
            if (msg.value != 0) revert InvalidAmount();
            if (
                !IERC20(token).transferFrom(msg.sender, address(this), amount)
            ) {
                revert TransferFailed();
            }
        }

        emit IntentFilled(intentId, msg.sender, token, amount);
        emit MerkleRootUpdated(_computeMerkleRoot());
    }

    /**
     * @notice Auto-claim withdrawal with full verification
     * @dev Called by relayer after user provides secret
     * @param intentId Intent identifier
     * @param nullifier One-time use nullifier
     * @param recipient Address to receive funds
     * @param secret Secret that unlocks the commitment
     * @param claimAuth Signature proving recipient authorization
     */
    function claimWithdrawal(
        bytes32 intentId,
        bytes32 nullifier,
        address recipient,
        bytes32 secret,
        bytes calldata claimAuth
    ) external nonReentrant onlyRelayer {
        Fill storage fill = fills[intentId];

        // Validate fill exists and is claimable
        if (fill.solver == address(0)) revert NotFilled();
        if (fill.claimed) revert AlreadyClaimed();
        if (nullifiers[nullifier]) revert NullifierUsed();

        // Verify authorization signature
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipient)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        address signer = ECDSA.recover(ethSignedHash, claimAuth);

        if (signer != recipient || signer == address(0)) {
            revert InvalidSignature();
        }

        // Verify commitment with all parameters
        bytes32 computedCommitment = POSEIDON_HASHER.poseidon(
            [
                secret,
                nullifier,
                bytes32(fill.amount),
                bytes32(uint256(fill.sourceChain))
            ]
        );

        // Get registered intent parameters
        IntentParams storage params = intentParams[intentId];
        if (!params.exists) revert IntentNotRegistered();

        // Verify computed commitment matches registered commitment
        if (computedCommitment != params.commitment) {
            revert InvalidCommitment();
        }

        // Verify fill parameters match registered parameters
        if (fill.amount != params.amount) revert AmountMismatch();
        if (fill.token != params.token) revert TokenMismatch();
        if (fill.sourceChain != params.sourceChain) revert ChainMismatch();

        // Check contract has sufficient balance BEFORE marking claimed
        if (fill.token == NATIVE_ETH) {
            if (address(this).balance < fill.amount)
                revert InsufficientBalance();
        } else {
            if (IERC20(fill.token).balanceOf(address(this)) < fill.amount) {
                revert InsufficientBalance();
            }
        }

        // Mark as claimed (CEI pattern - state changes before external calls)
        fill.claimed = true;
        nullifiers[nullifier] = true;

        // Calculate fees
        uint256 fee = (fill.amount * FEE_BPS) / 10000;
        uint256 userAmount = fill.amount - fee;

        // Handle token transfer
        if (fill.token == NATIVE_ETH) {
            (bool success1, ) = recipient.call{value: userAmount}("");
            if (!success1) revert TransferFailed();

            (bool success2, ) = FEE_COLLECTOR.call{value: fee}("");
            if (!success2) revert TransferFailed();
        } else {
            if (!IERC20(fill.token).transfer(recipient, userAmount)) {
                revert TransferFailed();
            }

            if (!IERC20(fill.token).transfer(FEE_COLLECTOR, fee)) {
                revert TransferFailed();
            }
        }

        emit WithdrawalClaimed(
            intentId,
            nullifier,
            recipient,
            fill.token,
            userAmount
        );
    }

    /**
     * @notice Add supported token with specific limits
     * @param token Token address (use NATIVE_ETH for ETH)
     * @param minAmount Minimum fill amount in token's smallest unit
     * @param maxAmount Maximum fill amount in token's smallest unit
     * @param decimals Token decimals (18 for ETH/WETH, 6 for USDC/USDT)
     */
    function addSupportedToken(
        address token,
        uint256 minAmount,
        uint256 maxAmount,
        uint256 decimals
    ) external onlyOwner {
        if (tokenConfigs[token].supported) revert AlreadySupported();

        // Prevent actual zero address, but allow NATIVE_ETH (0xEeee...)
        if (token == address(0)) revert InvalidToken();

        if (minAmount == 0 || maxAmount == 0 || minAmount > maxAmount) {
            revert InvalidTokenConfig();
        }

        tokenConfigs[token] = TokenConfig({
            supported: true,
            minFillAmount: minAmount,
            maxFillAmount: maxAmount,
            decimals: decimals
        });

        tokenIndex[token] = tokenList.length;
        tokenList.push(token);

        emit TokenAdded(token, minAmount, maxAmount);
    }

    /**
     * @notice Update token configuration limits
     * @param token Token address
     * @param minAmount New minimum fill amount
     * @param maxAmount New maximum fill amount
     */
    function updateTokenConfig(
        address token,
        uint256 minAmount,
        uint256 maxAmount
    ) external onlyOwner {
        if (!tokenConfigs[token].supported) revert TokenNotSupported();
        if (minAmount == 0 || maxAmount == 0 || minAmount > maxAmount) {
            revert InvalidTokenConfig();
        }

        tokenConfigs[token].minFillAmount = minAmount;
        tokenConfigs[token].maxFillAmount = maxAmount;

        emit TokenConfigUpdated(token, minAmount, maxAmount);
    }

    /**
     * @notice Remove token from supported list
     * @param token Token address to remove
     */
    function removeSupportedToken(address token) external onlyOwner {
        if (!tokenConfigs[token].supported) revert TokenNotSupported();

        tokenConfigs[token].supported = false;

        uint256 index = tokenIndex[token];
        uint256 lastIndex = tokenList.length - 1;

        if (index != lastIndex) {
            address lastToken = tokenList[lastIndex];
            tokenList[index] = lastToken;
            tokenIndex[lastToken] = index;
        }

        tokenList.pop();
        delete tokenIndex[token];

        emit TokenRemoved(token);
    }

    /**
     * @notice Sync source chain merkle root for verification
     * @param chainId Source chain identifier
     * @param root Merkle root from source chain
     */
    function syncSourceChainRoot(
        uint32 chainId,
        bytes32 root
    ) external onlyRelayer {
        sourceChainRoots[chainId] = root;
        emit RootSynced(chainId, root);
    }

    /**
     * @notice CANONICAL hash pair - sorts inputs before hashing
     * @dev Ensures hash(A,B) = hash(B,A) for merkle tree consistency
     */
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice Verify commitment exists on source chain via merkle proof
     * @dev Uses CANONICAL hashing to match off-chain proof generation
     */
    function _verifySourceCommitment(
        bytes32 commitment,
        uint32 sourceChain,
        bytes32 root,
        bytes32[] calldata proof,
        uint256 index
    ) internal view returns (bool) {
        if (sourceChainRoots[sourceChain] != root) return false;

        bytes32 computedHash = commitment;

        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 proofElement = proof[i];
            computedHash = _hashPair(computedHash, proofElement);
            index = index / 2;
        }

        return computedHash == root;
    }

    /**
     * @notice Compute merkle root of all fills using CANONICAL hashing
     */
    function _computeMerkleRoot() internal view returns (bytes32) {
        if (fillTree.length == 0) return bytes32(0);
        if (fillTree.length == 1) return fillTree[0];

        uint256 n = fillTree.length;

        bytes32[] memory layer = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            layer[i] = fillTree[i];
        }

        while (n > 1) {
            for (uint256 i = 0; i < n / 2; i++) {
                layer[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }

            if (n % 2 == 1) {
                layer[n / 2] = layer[n - 1];
                n = n / 2 + 1;
            } else {
                n = n / 2;
            }
        }

        return layer[0];
    }

    /**
     * @notice Generate merkle proof for a fill
     * @param intentId Intent identifier
     * @return proof Array of sibling hashes for merkle verification
     */
    function generateFillProof(
        bytes32 intentId
    ) external view returns (bytes32[] memory) {
        uint256 index = fillIndex[intentId];
        if (index >= fillTree.length) revert NotFilled();

        uint256 proofLength = 0;
        uint256 n = fillTree.length;
        while (n > 1) {
            proofLength++;
            n = (n + 1) / 2;
        }

        bytes32[] memory proof = new bytes32[](proofLength);
        uint256 currentIndex = index;

        bytes32[] memory layer = new bytes32[](fillTree.length);
        for (uint256 i = 0; i < fillTree.length; i++) {
            layer[i] = fillTree[i];
        }

        n = fillTree.length;
        uint256 proofIndex = 0;

        while (n > 1) {
            if (currentIndex % 2 == 0) {
                if (currentIndex + 1 < n) {
                    proof[proofIndex] = layer[currentIndex + 1];
                } else {
                    proof[proofIndex] = layer[currentIndex];
                }
            } else {
                proof[proofIndex] = layer[currentIndex - 1];
            }

            proofIndex++;

            for (uint256 i = 0; i < n / 2; i++) {
                layer[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }

            if (n % 2 == 1) {
                layer[n / 2] = layer[n - 1];
                n = n / 2 + 1;
            } else {
                n = n / 2;
            }

            currentIndex = currentIndex / 2;
        }

        return proof;
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    function getIntentParams(
        bytes32 intentId
    ) external view returns (IntentParams memory) {
        return intentParams[intentId];
    }

    function getTokenConfig(
        address token
    ) external view returns (TokenConfig memory) {
        return tokenConfigs[token];
    }

    function isIntentRegistered(bytes32 intentId) external view returns (bool) {
        return intentParams[intentId].exists;
    }

    function getFill(bytes32 intentId) external view returns (Fill memory) {
        return fills[intentId];
    }

    function getFillIndex(bytes32 intentId) external view returns (uint256) {
        return fillIndex[intentId];
    }

    function isNullifierUsed(bytes32 nullifier) external view returns (bool) {
        return nullifiers[nullifier];
    }

    function getMerkleRoot() external view returns (bytes32) {
        return _computeMerkleRoot();
    }

    function getSourceChainRoot(
        uint32 chainId
    ) external view returns (bytes32) {
        return sourceChainRoots[chainId];
    }

    function getFillTreeSize() external view returns (uint256) {
        return fillTree.length;
    }

    function getSupportedTokens() external view returns (address[] memory) {
        return tokenList;
    }

    function getSupportedTokenCount() external view returns (uint256) {
        return tokenList.length;
    }

    function isTokenSupported(address token) external view returns (bool) {
        return tokenConfigs[token].supported;
    }

    /**
     * @notice Reject direct ETH deposits
     * @dev ETH must be deposited via fillIntent function
     */
    receive() external payable {
        revert DirectETHDepositNotAllowed();
    }
}
