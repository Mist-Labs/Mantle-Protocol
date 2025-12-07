// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IPoseidonHasher} from "./interface.sol";

/**
 * @title PrivateIntentPool
 * @notice Creates privacy-preserving cross-chain intents using commitments
 * @dev Uses CANONICAL Merkle hashing to match PrivateSettlement
 */
contract PrivateIntentPool is ReentrancyGuard {
    struct Intent {
        bytes32 commitment;
        address token;
        uint256 amount;
        uint32 destChain;
        uint32 deadline;
        address refundTo;
        bool filled;
        bool refunded;
    }

    mapping(bytes32 => Intent) public intents;
    mapping(bytes32 => bool) public commitments;
    mapping(uint32 => bytes32) public destChainRoots;
    mapping(bytes32 => address) public intentSolvers; 

    IPoseidonHasher poseidonHasher;
    address public immutable RELAYER;
    address public immutable FEE_COLLECTOR;

    uint256 public constant MIN_AMOUNT = 0.001 ether;
    uint256 public constant MAX_AMOUNT = 100 ether;
    uint256 public constant INTENT_TIMEOUT = 1 hours;
    uint256 public constant FEE_BPS = 10;

    event IntentCreated(
        bytes32 indexed intentId,
        bytes32 indexed commitment,
        uint32 destChain,
        uint256 amount
    );
    event IntentFilled(bytes32 indexed intentId, address indexed solver);
    event IntentRefunded(bytes32 indexed intentId);
    event RootSynced(uint32 indexed chainId, bytes32 root);

    error InvalidAmount();
    error DuplicateCommitment();
    error IntentNotFound();
    error IntentAlreadyFilled();
    error IntentNotExpired();
    error Unauthorized();
    error TransferFailed();
    error InvalidCommitment();
    error RootNotSynced();

    constructor(
        address _relayer,
        address _feeCollector,
        address _poseidonHasher
    ) {
        RELAYER = _relayer;
        FEE_COLLECTOR = _feeCollector;
        poseidonHasher = IPoseidonHasher(_poseidonHasher);
    }

    /**
     * @notice Create private intent with commitment verification
     */
    function createIntent(
        bytes32 intentId,
        bytes32 commitment,
        address token,
        uint256 amount,
        uint32 destChain,
        address refundTo,
        bytes32 secret,
        bytes32 nullifier
    ) external nonReentrant {
        if (msg.sender != RELAYER) revert Unauthorized();
        if (amount < MIN_AMOUNT || amount > MAX_AMOUNT) revert InvalidAmount();
        if (commitments[commitment]) revert DuplicateCommitment();
        if (intents[intentId].commitment != bytes32(0))
            revert DuplicateCommitment();

        bytes32 computedCommitment = poseidonHasher.poseidon(
            [secret, nullifier, bytes32(amount), bytes32(uint256(destChain))]
        );

        if (computedCommitment != commitment) revert InvalidCommitment();

        commitments[commitment] = true;

        intents[intentId] = Intent({
            commitment: commitment,
            token: token,
            amount: amount,
            destChain: destChain,
            deadline: uint32(block.timestamp + INTENT_TIMEOUT),
            refundTo: refundTo,
            filled: false,
            refunded: false
        });

        if (!IERC20(token).transferFrom(msg.sender, address(this), amount)) {
            revert TransferFailed();
        }

        emit IntentCreated(intentId, commitment, destChain, amount);
    }

    /**
     * @notice Mark intent as filled after cross-chain verification
     * @dev Any solver can call this function - first valid proof wins
     * @param intentId The unique identifier of the intent
     * @param merkleProof Merkle proof showing intent was fulfilled on destination chain
     * @param leafIndex Position of the leaf in the Merkle tree
     */
    function markFilled(
        bytes32 intentId,
        bytes32[] calldata merkleProof,
        uint256 leafIndex
    ) external nonReentrant {
        address solver = msg.sender;

        Intent storage intent = intents[intentId];

        // Validate intent exists and is still active
        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadyFilled();

        // Get the synced root for the destination chain
        bytes32 destRoot = destChainRoots[intent.destChain];
        if (destRoot == bytes32(0)) revert RootNotSynced();

        // Verify Merkle proof that intent was fulfilled on destination chain
        if (!_verifyMerkleProof(intentId, destRoot, merkleProof, leafIndex)) {
            revert InvalidCommitment();
        }

        // Mark intent as filled and record the solver
        intent.filled = true;
        intentSolvers[intentId] = solver;

        uint256 fee = (intent.amount * FEE_BPS) / 10000;
        uint256 solverAmount = intent.amount - fee;

        // Transfer tokens to solver
        if (!IERC20(intent.token).transfer(solver, solverAmount)) {
            revert TransferFailed();
        }

        if (!IERC20(intent.token).transfer(FEE_COLLECTOR, fee)) {
            revert TransferFailed();
        }

        emit IntentFilled(intentId, solver);
    }

    /**
     * @notice Sync destination chain root for verification
     */
    function syncDestChainRoot(uint32 chainId, bytes32 root) external {
        if (msg.sender != RELAYER) revert Unauthorized();
        destChainRoots[chainId] = root;
        emit RootSynced(chainId, root);
    }

    /**
     * @notice Refund expired intent
     */
    function refund(bytes32 intentId) external nonReentrant {
        Intent storage intent = intents[intentId];

        if (intent.commitment == bytes32(0)) revert IntentNotFound();
        if (intent.filled || intent.refunded) revert IntentAlreadyFilled();
        if (block.timestamp < intent.deadline) revert IntentNotExpired();

        intent.refunded = true;

        if (!IERC20(intent.token).transfer(intent.refundTo, intent.amount)) {
            revert TransferFailed();
        }

        emit IntentRefunded(intentId);
    }

    /**
     * @notice CANONICAL hash pair - sorts inputs before hashing
     * @dev Matches PrivateSettlement's hashing for compatibility
     */
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return a < b
            ? keccak256(abi.encodePacked(a, b))
            : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice Verify merkle proof using CANONICAL hashing
     * @dev MUST match PrivateSettlement's proof generation
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

    function getIntent(bytes32 intentId) external view returns (Intent memory) {
        return intents[intentId];
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
}