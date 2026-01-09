// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {
    MerkleProof
} from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
import {IPoseidonHasher} from "./interface.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title PrivateIntentPool (Source Chain Contract)
 * @notice FIXED: Power-of-2 zero padding + OpenZeppelin MerkleProof
 * @dev Eliminates orphan edge cases, uses battle-tested OZ verification
 * @custom:security-contact ebounce500@gmail.com
 */
contract PrivateIntentPool is ReentrancyGuard, Ownable {
    struct Intent {
        bytes32 commitment;
        address sourceToken;
        uint256 sourceAmount;
        address destToken;
        uint256 destAmount;
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
    mapping(uint32 => bytes32) public destChainFillRoots;
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
        address sourceToken,
        uint256 sourceAmount,
        address destToken,
        uint256 destAmount
    );
    event IntentSettled(
        bytes32 indexed intentId,
        address indexed solver,
        bytes32 fillRoot
    );
    event IntentRefunded(bytes32 indexed intentId, uint256 amount);
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event FillRootSynced(uint32 indexed chainId, bytes32 root);
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
    error IntentAlreadySettled();
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
     * @notice Create private intent - ONLY escrows SOURCE tokens
     * @dev destToken and destAmount are INFORMATIONAL for solver to read from events
     * @param intentId Unique identifier for the intent
     * @param commitment Privacy commitment hash
     * @param sourceToken Token address on THIS chain (use NATIVE_ETH for ETH)
     * @param sourceAmount Amount to ESCROW on THIS chain
     * @param destToken INFORMATIONAL: Token address on destination chain
     * @param destAmount INFORMATIONAL: Amount solver should pay on destination
     * @param destChain Destination chain ID
     * @param refundTo Address to receive refund if intent expires
     * @param customDeadline Optional custom deadline (0 = use default timeout)
     */
    function createIntent(
        bytes32 intentId,
        bytes32 commitment,
        address sourceToken,
        uint256 sourceAmount,
        address destToken,
        uint256 destAmount,
        uint32 destChain,
        address refundTo,
        uint64 customDeadline
    ) external payable nonReentrant {
        TokenConfig storage config = tokenConfigs[sourceToken];
        if (!config.supported) revert TokenNotSupported();
        if (
            sourceAmount < config.minFillAmount ||
            sourceAmount > config.maxFillAmount
        ) revert InvalidAmount();
        if (destAmount == 0) revert InvalidAmount();
        if (commitments[commitment]) revert DuplicateCommitment();
        if (intents[intentId].commitment != bytes32(0))
            revert DuplicateCommitment();
        if (refundTo == address(0)) revert InvalidAddress();

        uint64 deadline;
        if (customDeadline == 0) {
            deadline = uint64(block.timestamp + DEFAULT_INTENT_TIMEOUT);
        } else {
            if (customDeadline <= block.timestamp) revert InvalidDeadline();
            deadline = customDeadline;
        }

        commitments[commitment] = true;

        intents[intentId] = Intent({
            commitment: commitment,
            sourceToken: sourceToken,
            sourceAmount: sourceAmount,
            destToken: destToken,
            destAmount: destAmount,
            destChain: destChain,
            deadline: deadline,
            refundTo: refundTo,
            filled: false,
            refunded: false
        });

        commitmentTree.push(commitment);
        commitmentIndex[commitment] = commitmentTree.length - 1;

        if (sourceToken == NATIVE_ETH) {
            if (msg.value != sourceAmount) revert InvalidAmount();
        } else {
            if (msg.value != 0) revert InvalidAmount();
            if (
                !IERC20(sourceToken).transferFrom(
                    msg.sender,
                    address(this),
                    sourceAmount
                )
            ) revert TransferFailed();
        }

        emit IntentCreated(
            intentId,
            commitment,
            destChain,
            sourceToken,
            sourceAmount,
            destToken,
            destAmount
        );
        emit MerkleRootUpdated(_computeMerkleRoot());
    }

    /**
     * @notice Settle intent after destination fill is verified
     * @dev Called by relayer after solver filled on destination chain
     * @dev Releases SOURCE tokens to reimburse solver for their destination payout
     * @dev Uses OpenZeppelin's MerkleProof.verify for security
     * @param intentId The unique identifier of the intent
     * @param solver Address of solver who filled on destination
     * @param merkleProof Proof that intentId exists in destination fillTree
     * @param leafIndex Position of intentId in destination merkle tree (unused with sorted hashing)
     */
    function settleIntent(
        bytes32 intentId,
        address solver,
        bytes32[] calldata merkleProof,
        uint256 leafIndex
    ) external nonReentrant onlyRelayer {
        Intent storage intent = intents[intentId];
        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadySettled();
        if (solver == address(0)) revert InvalidAddress();

        bytes32 destRoot = destChainRoots[intent.destChain];
        if (destRoot == bytes32(0)) revert RootNotSynced();

        // Use OpenZeppelin's battle-tested verification
        if (!MerkleProof.verify(merkleProof, destRoot, intentId)) {
            revert InvalidProof();
        }

        if (intent.sourceToken == NATIVE_ETH) {
            if (address(this).balance < intent.sourceAmount)
                revert InsufficientBalance();
        } else {
            if (
                IERC20(intent.sourceToken).balanceOf(address(this)) <
                intent.sourceAmount
            ) revert InsufficientBalance();
        }

        intent.filled = true;
        intentSolvers[intentId] = solver;

        uint256 fee = (intent.sourceAmount * FEE_BPS) / 10000;
        uint256 solverReimbursement = intent.sourceAmount - fee;

        if (intent.sourceToken == NATIVE_ETH) {
            (bool success1, ) = solver.call{value: solverReimbursement}("");
            if (!success1) revert TransferFailed();
            (bool success2, ) = FEE_COLLECTOR.call{value: fee}("");
            if (!success2) revert TransferFailed();
        } else {
            if (
                !IERC20(intent.sourceToken).transfer(
                    solver,
                    solverReimbursement
                )
            ) revert TransferFailed();
            if (!IERC20(intent.sourceToken).transfer(FEE_COLLECTOR, fee))
                revert TransferFailed();
        }

        emit IntentSettled(intentId, solver, destRoot);
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
     * @notice Sync destination chain FILL merkle root
     * @dev Used to verify that an intent was actually filled on the destination
     * @param chainId Destination chain identifier
     * @param root Merkle root from destination chain's fillTree
     */
    function syncDestChainFillRoot(
        uint32 chainId,
        bytes32 root
    ) external onlyRelayer {
        destChainFillRoots[chainId] = root;
        emit FillRootSynced(chainId, root);
    }

    /**
     * @notice Refund expired intent to original depositor
     * @dev Can be called by anyone after deadline passes
     * @param intentId Intent identifier to refund
     */
    function refund(bytes32 intentId) external nonReentrant {
        Intent storage intent = intents[intentId];
        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadySettled();
        if (block.timestamp < intent.deadline) revert IntentNotExpired();

        if (intent.sourceToken == NATIVE_ETH) {
            if (address(this).balance < intent.sourceAmount)
                revert InsufficientBalance();
        } else {
            if (
                IERC20(intent.sourceToken).balanceOf(address(this)) <
                intent.sourceAmount
            ) revert InsufficientBalance();
        }

        intent.refunded = true;

        if (intent.sourceToken == NATIVE_ETH) {
            (bool success, ) = intent.refundTo.call{value: intent.sourceAmount}(
                ""
            );
            if (!success) revert TransferFailed();
        } else {
            if (
                !IERC20(intent.sourceToken).transfer(
                    intent.refundTo,
                    intent.sourceAmount
                )
            ) revert TransferFailed();
        }

        emit IntentRefunded(intentId, intent.sourceAmount);
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

    /**
     * @notice Update token configuration limits
     */
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
     * @notice FIXED: Canonical sorted hashing (compatible with OZ MerkleProof)
     */
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice FIXED: Power-of-2 zero padding eliminates orphan edge cases
     * @dev Tree size is always padded to next power of 2
     * @dev Every node has a sibling (even if it's bytes32(0))
     * @dev No more orphan promotion bugs!
     */
    function _computeMerkleRoot() internal view returns (bytes32) {
        uint256 n = commitmentTree.length;
        if (n == 0) return bytes32(0);
        if (n == 1) return commitmentTree[0];

        // Pad to next power of 2
        uint256 treeSize = _nextPowerOf2(n);
        bytes32[] memory layer = new bytes32[](treeSize);

        // Copy actual leaves
        for (uint256 i = 0; i < n; i++) {
            layer[i] = commitmentTree[i];
        }

        // Pad with zeros
        for (uint256 i = n; i < treeSize; i++) {
            layer[i] = bytes32(0);
        }

        // Build tree bottom-up
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
     * @dev Proof generation matches tree building exactly
     * @dev Uses same power-of-2 padding as _computeMerkleRoot
     * @param commitment Commitment to generate proof for
     * @return proof Array of sibling hashes
     * @return index Position in tree
     */
    function generateCommitmentProof(
        bytes32 commitment
    ) external view returns (bytes32[] memory proof, uint256 index) {
        index = commitmentIndex[commitment];
        if (index >= commitmentTree.length) revert IntentNotFound();

        uint256 n = commitmentTree.length;
        uint256 treeSize = _nextPowerOf2(n);

        // Calculate tree height
        uint256 height = 0;
        uint256 temp = treeSize;
        while (temp > 1) {
            height++;
            temp = temp / 2;
        }

        proof = new bytes32[](height);

        // Build initial layer with padding
        bytes32[] memory layer = new bytes32[](treeSize);
        for (uint256 i = 0; i < n; i++) {
            layer[i] = commitmentTree[i];
        }
        for (uint256 i = n; i < treeSize; i++) {
            layer[i] = bytes32(0);
        }

        uint256 currentIndex = index;
        uint256 currentSize = treeSize;

        // Generate proof level by level
        for (uint256 level = 0; level < height; level++) {
            // Sibling is always at currentIndex XOR 1
            uint256 siblingIndex = currentIndex ^ 1;
            proof[level] = layer[siblingIndex];

            // Build next layer
            for (uint256 i = 0; i < currentSize / 2; i++) {
                layer[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }

            currentIndex = currentIndex / 2;
            currentSize = currentSize / 2;
        }

        return (proof, index);
    }

    /**
     * @notice Calculate next power of 2 >= n
     * @dev Used for power-of-2 tree padding
     */
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

    // ========== VIEW FUNCTIONS ==========

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

    receive() external payable {
        revert DirectETHDepositNotAllowed();
    }
}
