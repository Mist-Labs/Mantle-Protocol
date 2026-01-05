// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IPoseidonHasher} from "./interface.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title PrivateIntentPool
 * @notice Creates privacy-preserving cross-chain intents using commitments
 * @dev Multi-token support, proper validation, matches PrivateSettlement
 * @custom:security-contact ebounce500@gmail.com
 */
contract PrivateIntentPool is ReentrancyGuard, Ownable {
    struct Intent {
        bytes32 commitment;
        address token;
        uint256 amount;
        uint32 destChain;
        uint64 deadline;
        address refundTo;
        bool filled;
        bool refunded;
    }

    struct TokenConfig {
        bool supported;
        uint256 minFillAmount;
        uint256 maxFillAmount;
        uint256 decimals;
    }

    mapping(bytes32 => Intent) public intents;
    mapping(bytes32 => bool) public commitments;
    mapping(uint32 => bytes32) public destChainRoots;
    mapping(bytes32 => address) public intentSolvers;
    mapping(address => TokenConfig) public tokenConfigs;

    address[] private tokenList;
    mapping(address => uint256) private tokenIndex;

    bytes32[] public commitmentTree;
    mapping(bytes32 => uint256) public commitmentIndex;

    IPoseidonHasher public immutable POSEIDON_HASHER;
    address public immutable RELAYER;
    address public immutable FEE_COLLECTOR;

    uint256 public constant DEFAULT_INTENT_TIMEOUT = 6 hours;
    uint256 public constant FEE_BPS = 20; // 0.2%
    address public constant NATIVE_ETH =
        address(0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE);

    event IntentCreated(
        bytes32 indexed intentId,
        bytes32 indexed commitment,
        uint32 destChain,
        address token,
        uint256 amount
    );
    event IntentMarkedFilled(bytes32 indexed intentId, address indexed solver, bytes32 fillRoot);
    event IntentRefunded(bytes32 indexed intentId, uint256 amount);
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

    error InvalidAmount();
    error InvalidToken();
    error DuplicateCommitment();
    error IntentNotFound();
    error IntentAlreadyFilled();
    error IntentNotExpired();
    error Unauthorized();
    error TransferFailed();
    error InvalidProof();
    error RootNotSynced();
    error AlreadySupported();
    error TokenNotSupported();
    error InvalidTokenConfig();
    error InsufficientBalance();
    error InvalidDeadline();
    error InvalidAddress();
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
     * @notice Create private intent with commitment verification
     * @dev Supports both native ETH and ERC20 tokens
     * @param intentId Unique identifier for the intent
     * @param commitment Privacy commitment (hash of secret + nullifier + amount + chain)
     * @param token Token address (use NATIVE_ETH for native transfers)
     * @param amount Amount in token's smallest unit
     * @param destChain Destination chain ID
     * @param refundTo Address to receive refund if intent expires
     * @param customDeadline Optional custom deadline (0 = use default timeout)
     */
    function createIntent(
        bytes32 intentId,
        bytes32 commitment,
        address token,
        uint256 amount,
        uint32 destChain,
        address refundTo,
        uint64 customDeadline
    ) external payable nonReentrant {
        // Validate token configuration
        TokenConfig storage config = tokenConfigs[token];
        if (!config.supported) revert TokenNotSupported();
        if (amount < config.minFillAmount || amount > config.maxFillAmount) {
            revert InvalidAmount();
        }

        // Validate commitment and intent ID
        if (commitments[commitment]) revert DuplicateCommitment();
        if (intents[intentId].commitment != bytes32(0)) {
            revert DuplicateCommitment();
        }

        if (refundTo == address(0)) revert InvalidToken();

        //  Calculate deadline
        uint64 deadline;
        if (customDeadline == 0) {
            deadline = uint64(block.timestamp + DEFAULT_INTENT_TIMEOUT);
        } else {
            if (customDeadline <= block.timestamp) revert InvalidDeadline();
            deadline = customDeadline;
        }

        // Mark commitment as used
        commitments[commitment] = true;

        // Store intent
        intents[intentId] = Intent({
            commitment: commitment,
            token: token,
            amount: amount,
            destChain: destChain,
            deadline: deadline,
            refundTo: refundTo,
            filled: false,
            refunded: false
        });

        // Add to merkle tree
        commitmentTree.push(commitment);
        commitmentIndex[commitment] = commitmentTree.length - 1;

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

        emit IntentCreated(intentId, commitment, destChain, token, amount);
        emit MerkleRootUpdated(_computeMerkleRoot());
    }

    /**
     * @notice Mark intent as filled after cross-chain verification
     * @dev Called by relayer after filling on destination chain - proves fill happened
     * @param intentId The unique identifier of the intent
     * @param merkleProof Merkle proof showing intentId exists in destination fillTree
     * @param leafIndex Position of the intentId in destination merkle tree
     */
    function markFilled(
        bytes32 intentId,
        address solver,
        bytes32[] calldata merkleProof,
        uint256 leafIndex
    ) external nonReentrant {
        Intent storage intent = intents[intentId];

        // Validate intent state
        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadyFilled();
        if (solver == address(0)) revert InvalidAddress();

        // Get synced root from destination chain
        bytes32 destRoot = destChainRoots[intent.destChain];
        if (destRoot == bytes32(0)) revert RootNotSynced();

        // Verify intentId (not commitment) exists in dest chain fillTree
        if (!_verifyMerkleProof(intentId, destRoot, merkleProof, leafIndex)) {
            revert InvalidProof();
        }

        // Check balance before state changes
        if (intent.token == NATIVE_ETH) {
            if (address(this).balance < intent.amount)
                revert InsufficientBalance();
        } else {
            if (IERC20(intent.token).balanceOf(address(this)) < intent.amount) {
                revert InsufficientBalance();
            }
        }

        // Mark as filled (CEI pattern)
        intent.filled = true;
        intentSolvers[intentId] = solver;

        // Calculate distribution
        uint256 fee = (intent.amount * FEE_BPS) / 10000;
        uint256 solverAmount = intent.amount - fee;

        // Transfer to solver and fee collector
        if (intent.token == NATIVE_ETH) {
            (bool success1, ) = solver.call{value: solverAmount}("");
            if (!success1) revert TransferFailed();

            (bool success2, ) = FEE_COLLECTOR.call{value: fee}("");
            if (!success2) revert TransferFailed();
        } else {
            if (!IERC20(intent.token).transfer(solver, solverAmount)) {
                revert TransferFailed();
            }

            if (!IERC20(intent.token).transfer(FEE_COLLECTOR, fee)) {
                revert TransferFailed();
            }
        }

        emit IntentMarkedFilled(intentId, solver, _computeMerkleRoot()); 
    }

    /**
     * @notice Sync destination chain merkle root from settlement contract
     * @dev Called by relayer to update proof verification root
     * @param chainId Destination chain identifier
     * @param root Merkle root from destination chain's PrivateSettlement.fillTree
     */
    function syncDestChainRoot(
        uint32 chainId,
        bytes32 root
    ) external onlyRelayer {
        destChainRoots[chainId] = root;
        emit RootSynced(chainId, root);
    }

    /**
     * @notice Refund expired intent to original depositor
     * @dev Can be called by anyone after deadline passes
     * @param intentId Intent identifier to refund
     */
    function refund(bytes32 intentId) external nonReentrant {
        Intent storage intent = intents[intentId];

        // Validate intent state
        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadyFilled();
        if (block.timestamp < intent.deadline) revert IntentNotExpired();

        // Check balance before refund
        if (intent.token == NATIVE_ETH) {
            if (address(this).balance < intent.amount)
                revert InsufficientBalance();
        } else {
            if (IERC20(intent.token).balanceOf(address(this)) < intent.amount) {
                revert InsufficientBalance();
            }
        }

        // Mark as refunded (CEI pattern)
        intent.refunded = true;

        // Transfer back to refundTo address
        if (intent.token == NATIVE_ETH) {
            (bool success, ) = intent.refundTo.call{value: intent.amount}("");
            if (!success) revert TransferFailed();
        } else {
            if (
                !IERC20(intent.token).transfer(intent.refundTo, intent.amount)
            ) {
                revert TransferFailed();
            }
        }

        emit IntentRefunded(intentId, intent.amount);
    }

    /**
     * @notice Add supported token with specific limits
     * @param token Token address (use NATIVE_ETH for native transfers)
     * @param minAmount Minimum intent amount in token's smallest unit
     * @param maxAmount Maximum intent amount in token's smallest unit
     * @param decimals Token decimals (18 for ETH/WETH, 6 for USDC/USDT)
     */
    function addSupportedToken(
        address token,
        uint256 minAmount,
        uint256 maxAmount,
        uint256 decimals
    ) external onlyOwner {
        if (tokenConfigs[token].supported) revert AlreadySupported();
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
     * @notice CANONICAL hash pair - sorts inputs before hashing
     * @dev MUST match PrivateSettlement's hashing for cross-chain compatibility
     */
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice Verify merkle proof using CANONICAL hashing
     * @dev Verifies intentId exists in destination chain's fillTree
     */
    function _verifyMerkleProof(
        bytes32 leaf,
        bytes32 root,
        bytes32[] calldata proof,
        uint256 index
    ) internal pure returns (bool) {
        bytes32 computedHash = leaf;

        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 proofElement = proof[i];
            computedHash = _hashPair(computedHash, proofElement);
            index = index / 2;
        }

        return computedHash == root;
    }

    /**
     * @notice Compute merkle root of all commitments using CANONICAL hashing
     * @dev Used for generating proofs that can be verified on destination chain
     */
    function _computeMerkleRoot() internal view returns (bytes32) {
        if (commitmentTree.length == 0) return bytes32(0);
        if (commitmentTree.length == 1) return commitmentTree[0];

        uint256 n = commitmentTree.length;

        bytes32[] memory layer = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            layer[i] = commitmentTree[i];
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
     * @notice Generate merkle proof for a commitment
     * @dev Called by RELAYER to prove commitment exists on source chain
     * @dev This proof is used when calling PrivateSettlement.fillIntent() on destination
     * @param commitment Commitment to generate proof for
     * @return proof Array of sibling hashes
     * @return index Position in tree
     */
    function generateCommitmentProof(
        bytes32 commitment
    ) external view returns (bytes32[] memory proof, uint256 index) {
        index = commitmentIndex[commitment];
        if (index >= commitmentTree.length) revert IntentNotFound();

        uint256 proofLength = 0;
        uint256 n = commitmentTree.length;
        while (n > 1) {
            proofLength++;
            n = (n + 1) / 2;
        }

        proof = new bytes32[](proofLength);
        uint256 currentIndex = index;

        bytes32[] memory layer = new bytes32[](commitmentTree.length);
        for (uint256 i = 0; i < commitmentTree.length; i++) {
            layer[i] = commitmentTree[i];
        }

        n = commitmentTree.length;
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

        return (proof, index);
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    function getIntent(bytes32 intentId) external view returns (Intent memory) {
        return intents[intentId];
    }

    function getTokenConfig(
        address token
    ) external view returns (TokenConfig memory) {
        return tokenConfigs[token];
    }

    function isCommitmentUsed(bytes32 commitment) external view returns (bool) {
        return commitments[commitment];
    }

    function getDestChainRoot(uint32 chainId) external view returns (bytes32) {
        return destChainRoots[chainId];
    }

    function getSolver(bytes32 intentId) external view returns (address) {
        return intentSolvers[intentId];
    }

    function getMerkleRoot() external view returns (bytes32) {
        return _computeMerkleRoot();
    }

    function getCommitmentTreeSize() external view returns (uint256) {
        return commitmentTree.length;
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
     * @dev ETH must be deposited via createIntent function
     */
    receive() external payable {
        revert DirectETHDepositNotAllowed();
    }
}
