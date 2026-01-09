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
import {
    MerkleProof
} from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
import {IPoseidonHasher} from "./interface.sol";

/**
 * @title PrivateSettlement (Destination Chain Contract)
 * @notice FIXED: Power-of-2 zero padding + OpenZeppelin MerkleProof
 * @dev Eliminates orphan edge cases, uses battle-tested OZ verification
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
    mapping(uint32 => bytes32) public sourceChainCommitmentRoots;
    mapping(bytes32 => IntentParams) public intentParams;
    mapping(address => TokenConfig) public tokenConfigs;

    address[] private tokenList;
    mapping(address => uint256) private tokenIndex;

    bytes32[] public fillTree;
    mapping(bytes32 => uint256) public fillIndex;

    IPoseidonHasher public immutable POSEIDON_HASHER;
    address public immutable RELAYER;
    address public immutable FEE_COLLECTOR;

    uint256 public constant FEE_BPS = 20;
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
        address token
    );
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event CommitmentRootSynced(uint32 indexed chainId, bytes32 root);
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
        address destToken,
        uint256 destAmount,
        uint32 sourceChain,
        uint64 deadline,
        bytes32[] proof,
        uint256 leafIndex
    );

    error InvalidProof();
    error InvalidToken();
    error NullifierUsed();
    error AlreadyFilled();
    error AlreadyRegistered();
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
     * @notice Register intent using OZ MerkleProof verification
     */
    function registerIntent(
        bytes32 intentId,
        bytes32 commitment,
        address destToken,
        uint256 destAmount,
        uint32 sourceChain,
        uint64 deadline,
        bytes32 sourceRoot,
        bytes32[] calldata proof,
        uint256 leafIndex
    ) external onlyRelayer {
        if (intentParams[intentId].exists) revert AlreadyRegistered();

        TokenConfig storage config = tokenConfigs[destToken];
        if (!config.supported) revert TokenNotSupported();
        if (
            destAmount < config.minFillAmount ||
            destAmount > config.maxFillAmount
        ) revert InvalidAmount();

        // Verify commitment exists on source chain using OZ
        if (!MerkleProof.verify(proof, sourceRoot, commitment)) {
            revert InvalidProof();
        }

        // Verify synced root matches
        if (sourceChainRoots[sourceChain] != sourceRoot) revert InvalidProof();

        intentParams[intentId] = IntentParams({
            commitment: commitment,
            token: destToken,
            amount: destAmount,
            sourceChain: sourceChain,
            deadline: deadline,
            exists: true
        });

        emit IntentRegistered(
            intentId,
            commitment,
            destToken,
            destAmount,
            sourceChain,
            deadline,
            proof,
            leafIndex
        );
    }

    function fillIntent(
        bytes32 intentId,
        bytes32 commitment,
        uint32 sourceChain,
        address token,
        uint256 amount
    ) external payable nonReentrant {
        if (fills[intentId].solver != address(0)) revert AlreadyFilled();

        TokenConfig storage config = tokenConfigs[token];
        if (!config.supported) revert TokenNotSupported();

        IntentParams storage params = intentParams[intentId];
        if (!params.exists) revert IntentNotRegistered();

        if (block.timestamp > params.deadline) revert IntentExpired();

        if (params.commitment != commitment) revert InvalidCommitment();
        if (params.amount != amount) revert AmountMismatch();
        if (params.token != token) revert TokenMismatch();
        if (params.sourceChain != sourceChain) revert ChainMismatch();

        fills[intentId] = Fill({
            solver: msg.sender,
            token: token,
            amount: amount,
            sourceChain: sourceChain,
            timestamp: uint32(block.timestamp),
            claimed: false
        });

        fillTree.push(intentId);
        fillIndex[intentId] = fillTree.length - 1;

        if (token == NATIVE_ETH) {
            if (msg.value != amount) revert InvalidAmount();
        } else {
            if (msg.value != 0) revert InvalidAmount();
            if (!IERC20(token).transferFrom(msg.sender, address(this), amount))
                revert TransferFailed();
        }

        emit IntentFilled(intentId, msg.sender, token, amount);
        emit MerkleRootUpdated(_computeMerkleRoot());
    }

    function claimWithdrawal(
        bytes32 intentId,
        bytes32 nullifier,
        address recipient,
        bytes32 secret,
        bytes calldata claimAuth
    ) external nonReentrant onlyRelayer {
        Fill storage fill = fills[intentId];

        if (fill.solver == address(0)) revert NotFilled();
        if (fill.claimed) revert AlreadyClaimed();
        if (nullifiers[nullifier]) revert NullifierUsed();

        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipient)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        address signer = ECDSA.recover(ethSignedHash, claimAuth);

        if (signer != recipient || signer == address(0))
            revert InvalidSignature();

        bytes32 computedCommitment = POSEIDON_HASHER.poseidon(
            [
                secret,
                nullifier,
                bytes32(fill.amount),
                bytes32(uint256(fill.sourceChain))
            ]
        );

        IntentParams storage params = intentParams[intentId];
        if (!params.exists) revert IntentNotRegistered();

        if (computedCommitment != params.commitment) revert InvalidCommitment();

        if (fill.amount != params.amount) revert AmountMismatch();
        if (fill.token != params.token) revert TokenMismatch();
        if (fill.sourceChain != params.sourceChain) revert ChainMismatch();

        if (fill.token == NATIVE_ETH) {
            if (address(this).balance < fill.amount)
                revert InsufficientBalance();
        } else {
            if (IERC20(fill.token).balanceOf(address(this)) < fill.amount)
                revert InsufficientBalance();
        }

        fill.claimed = true;
        nullifiers[nullifier] = true;

        uint256 fee = (fill.amount * FEE_BPS) / 10000;
        uint256 userAmount = fill.amount - fee;

        if (fill.token == NATIVE_ETH) {
            (bool success1, ) = recipient.call{value: userAmount}("");
            if (!success1) revert TransferFailed();
            (bool success2, ) = FEE_COLLECTOR.call{value: fee}("");
            if (!success2) revert TransferFailed();
        } else {
            if (!IERC20(fill.token).transfer(recipient, userAmount))
                revert TransferFailed();
            if (!IERC20(fill.token).transfer(FEE_COLLECTOR, fee))
                revert TransferFailed();
        }

        emit WithdrawalClaimed(intentId, nullifier, fill.token);
    }

    function addSupportedToken(
        address token,
        uint256 minAmount,
        uint256 maxAmount,
        uint256 decimals
    ) external onlyOwner {
        if (tokenConfigs[token].supported) revert AlreadySupported();
        if (token == address(0)) revert InvalidToken();
        if (minAmount == 0 || maxAmount == 0 || minAmount > maxAmount)
            revert InvalidTokenConfig();

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

    function updateTokenConfig(
        address token,
        uint256 minAmount,
        uint256 maxAmount
    ) external onlyOwner {
        if (!tokenConfigs[token].supported) revert TokenNotSupported();
        if (minAmount == 0 || maxAmount == 0 || minAmount > maxAmount)
            revert InvalidTokenConfig();

        tokenConfigs[token].minFillAmount = minAmount;
        tokenConfigs[token].maxFillAmount = maxAmount;

        emit TokenConfigUpdated(token, minAmount, maxAmount);
    }

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

    function syncSourceChainRoot(
        uint32 chainId,
        bytes32 root
    ) external onlyRelayer {
        sourceChainRoots[chainId] = root;
        emit RootSynced(chainId, root);
    }

    function syncSourceChainCommitmentRoot(
        uint32 chainId,
        bytes32 root
    ) external onlyRelayer {
        sourceChainCommitmentRoots[chainId] = root;
        emit CommitmentRootSynced(chainId, root);
    }

    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice FIXED: Power-of-2 zero padding (no orphans!)
     */
    function _computeMerkleRoot() internal view returns (bytes32) {
        uint256 n = fillTree.length;
        if (n == 0) return bytes32(0);
        if (n == 1) return fillTree[0];

        uint256 treeSize = _nextPowerOf2(n);
        bytes32[] memory layer = new bytes32[](treeSize);

        for (uint256 i = 0; i < n; i++) {
            layer[i] = fillTree[i];
        }
        for (uint256 i = n; i < treeSize; i++) {
            layer[i] = bytes32(0);
        }

        while (treeSize > 1) {
            for (uint256 i = 0; i < treeSize / 2; i++) {
                layer[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }
            treeSize = treeSize / 2;
        }

        return layer[0];
    }

    /**
     * @notice FIXED: Generate proof compatible with OZ MerkleProof.verify
     */
    function generateFillProof(
        bytes32 intentId
    ) external view returns (bytes32[] memory) {
        uint256 index = fillIndex[intentId];
        if (index >= fillTree.length) revert NotFilled();

        uint256 n = fillTree.length;
        uint256 treeSize = _nextPowerOf2(n);

        uint256 height = 0;
        uint256 temp = treeSize;
        while (temp > 1) {
            height++;
            temp = temp / 2;
        }

        bytes32[] memory proof = new bytes32[](height);

        bytes32[] memory layer = new bytes32[](treeSize);
        for (uint256 i = 0; i < n; i++) {
            layer[i] = fillTree[i];
        }
        for (uint256 i = n; i < treeSize; i++) {
            layer[i] = bytes32(0);
        }

        uint256 currentIndex = index;
        uint256 currentSize = treeSize;

        for (uint256 level = 0; level < height; level++) {
            uint256 siblingIndex = currentIndex ^ 1;
            proof[level] = layer[siblingIndex];

            for (uint256 i = 0; i < currentSize / 2; i++) {
                layer[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }

            currentIndex = currentIndex / 2;
            currentSize = currentSize / 2;
        }

        return proof;
    }

    function _nextPowerOf2(uint256 n) internal pure returns (uint256) {
        if (n == 0) return 1;
        n--;
        n |= n >> 1;
        n |= n >> 2;
        n |= n >> 4;
        n |= n >> 8;
        n |= n >> 16;
        n |= n >> 32;
        n |= n >> 64;
        n |= n >> 128;
        return n + 1;
    }

    // VIEW FUNCTIONS
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

    receive() external payable {
        revert DirectETHDepositNotAllowed();
    }
}
