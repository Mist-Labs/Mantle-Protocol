// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import {IPoseidonHasher} from "./interface.sol";

/**
 * @title PrivateSettlement
 * @notice Settles cross-chain intents with privacy-preserving auto-claims
 * @dev Uses CANONICAL Merkle hashing (sorted pairs) - industry standard
 */
contract PrivateSettlement is ReentrancyGuard {
    using ECDSA for bytes32;

    struct Fill {
        address solver;
        address token;
        uint256 amount;
        uint32 sourceChain;
        uint32 timestamp;
        bool claimed;
    }

    mapping(bytes32 => Fill) public fills;
    mapping(bytes32 => bool) public nullifiers;
    mapping(uint32 => bytes32) public sourceChainRoots;
    mapping(bytes32 => bytes32) public intentCommitments;

    // Merkle tree for fills (for source chain verification)
    bytes32[] public fillTree;
    mapping(bytes32 => uint256) public fillIndex;

    IPoseidonHasher public immutable poseidonHasher;
    address public immutable relayer;
    address public immutable feeCollector;

    uint256 public constant FEE_BPS = 5; // 0.05%

    event IntentFilled(
        bytes32 indexed intentId,
        address indexed solver,
        uint256 amount
    );
    event WithdrawalClaimed(
        bytes32 indexed intentId,
        bytes32 indexed nullifier,
        address recipient
    );
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event MerkleRootUpdated(bytes32 root);

    error InvalidProof();
    error NullifierUsed();
    error AlreadyFilled();
    error NotFilled();
    error AlreadyClaimed();
    error Unauthorized();
    error InvalidSignature();
    error TransferFailed();
    error InvalidCommitment();

    constructor(
        address _relayer,
        address _feeCollector,
        address _poseidonHasher
    ) {
        relayer = _relayer;
        feeCollector = _feeCollector;
        poseidonHasher = IPoseidonHasher(_poseidonHasher);
    }

    /**
     * @notice Solver fills intent by providing liquidity
     * @dev msg.sender is the SOLVER (not relayer)
     */
    function fillIntent(
        bytes32 intentId,
        bytes32 commitment,
        uint32 sourceChain,
        address token,
        uint256 amount,
        bytes32 sourceRoot,
        bytes32[] calldata merkleProof,
        uint256 leafIndex
    ) external nonReentrant {
        if (fills[intentId].solver != address(0)) revert AlreadyFilled();

        // Verify commitment exists on source chain
        if (
            !_verifySourceCommitment(
                commitment,
                sourceChain,
                sourceRoot,
                merkleProof,
                leafIndex
            )
        ) {
            revert InvalidProof();
        }

        fills[intentId] = Fill({
            solver: msg.sender,
            token: token,
            amount: amount,
            sourceChain: sourceChain,
            timestamp: uint32(block.timestamp),
            claimed: false
        });

        // Add to merkle tree for source chain verification
        fillTree.push(intentId);
        fillIndex[intentId] = fillTree.length - 1;

        // Store commitment for later verification
        intentCommitments[intentId] = commitment;

        if (!IERC20(token).transferFrom(msg.sender, address(this), amount)) {
            revert TransferFailed();
        }

        emit IntentFilled(intentId, msg.sender, amount);
        emit MerkleRootUpdated(_computeMerkleRoot());
    }

    /**
     * @notice Auto-claim withdrawal with full verification
     */
    function claimWithdrawal(
        bytes32 intentId,
        bytes32 nullifier,
        address recipient,
        bytes32 secret,
        bytes calldata claimAuth
    ) external nonReentrant {
        if (msg.sender != relayer) revert Unauthorized();

        Fill storage fill = fills[intentId];
        if (fill.solver == address(0)) revert NotFilled();
        if (fill.claimed) revert AlreadyClaimed();
        if (nullifiers[nullifier]) revert NullifierUsed();

        // Verify authorization signature (FIXED - use MessageHashUtils directly)
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipient)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        address signer = ECDSA.recover(ethSignedHash, claimAuth);

        if (signer != recipient) revert InvalidSignature();

        // Verify signer knows the secret/nullifier
        bytes32 computedCommitment = poseidonHasher.poseidon(
            [
                secret,
                nullifier,
                bytes32(fill.amount),
                bytes32(uint256(fill.sourceChain))
            ]
        );

        // CRITICAL: Verify computed commitment matches stored commitment
        if (computedCommitment != intentCommitments[intentId])
            revert InvalidCommitment();

        // Basic validation
        if (signer == address(0)) revert InvalidSignature();

        fill.claimed = true;
        nullifiers[nullifier] = true;

        uint256 fee = (fill.amount * FEE_BPS) / 10000;
        uint256 userAmount = fill.amount - fee;

        if (!IERC20(fill.token).transfer(recipient, userAmount)) {
            revert TransferFailed();
        }

        if (!IERC20(fill.token).transfer(feeCollector, fee)) {
            revert TransferFailed();
        }

        emit WithdrawalClaimed(intentId, nullifier, recipient);
    }

    /**
     * @notice Sync source chain root for verification
     */
    function syncSourceChainRoot(uint32 chainId, bytes32 root) external {
        if (msg.sender != relayer) revert Unauthorized();
        sourceChainRoots[chainId] = root;
        emit RootSynced(chainId, root);
    }

    /**
     * @notice CANONICAL hash pair - sorts inputs before hashing (industry standard)
     * @dev This ensures hash(A,B) = hash(B,A), preventing order-based attacks
     */
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    /**
     * @notice Verify commitment exists on source chain via merkle proof
     * @dev Uses CANONICAL hashing - must match off-chain proof generation
     */
    function _verifySourceCommitment(
        bytes32 commitment,
        uint32 sourceChain,
        bytes32 root,
        bytes32[] calldata proof,
        uint256 index
    ) internal view returns (bool) {
        // Verify root matches synced root
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
     * @dev Uses same _hashPair function for consistency
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
                // CANONICAL hashing for consistency
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
     * @notice Generate merkle proof for a fill using CANONICAL hashing
     * @dev Proof elements will be hashed canonically during verification
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

            // Build next layer using CANONICAL hashing
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

    function getFill(bytes32 intentId) external view returns (Fill memory) {
        return fills[intentId];
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
}
