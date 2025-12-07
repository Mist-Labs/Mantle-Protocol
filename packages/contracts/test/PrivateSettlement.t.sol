// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/poseidonHasher.sol";
import {PrivateSettlement} from "../src/privateSettlement.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract MockERC20 is ERC20 {
    constructor() ERC20("Mock Token", "MOCK") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/**
 * @title PrivateSettlementTest
 * @notice Comprehensive test suite for PrivateSettlement contract (uses canonical Merkle proofing)
 */
contract PrivateSettlementTest is Test {
    PoseidonHasher public poseidon;
    PrivateSettlement public settlement;
    MockERC20 public token;

    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public solver = makeAddr("solver");
    address public recipient = makeAddr("recipient");

    uint256 public recipientPrivateKey = 0x1234;
    address public recipientAddr;

    bytes32 public secret;
    bytes32 public nullifier;
    bytes32 public commitment;
    bytes32 public intentId;

    uint256 public constant TEST_AMOUNT = 1 ether;
    uint32 public constant SOURCE_CHAIN = 1;

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

    function setUp() public {
        // Deploy contracts
        poseidon = new PoseidonHasher();
        settlement = new PrivateSettlement(
            relayer,
            feeCollector,
            address(poseidon)
        );
        token = new MockERC20();

        // Setup recipient with known private key
        recipientAddr = vm.addr(recipientPrivateKey);

        // Setup test data
        secret = keccak256("secret");
        nullifier = keccak256("nullifier");
        intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        // Generate commitment
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        commitment = poseidon.poseidon(inputs);

        // Fund solver
        token.mint(solver, 1000 ether);
        vm.prank(solver);
        token.approve(address(settlement), type(uint256).max);
    }

    // ----------------------
    // Merkle helper functions used by tests
    // MUST match the contract's canonical ordering (a < b)
    // ----------------------

    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        // Uses the same canonical sorting as the contract
        return
            a < b
                ? keccak256(abi.encodePacked(a, b))
                : keccak256(abi.encodePacked(b, a));
    }

    function buildMerkleRootFromLeaves(
        bytes32[] memory leaves
    ) internal pure returns (bytes32) {
        uint256 n = leaves.length;
        if (n == 0) return bytes32(0);
        if (n == 1) return leaves[0];

        bytes32[] memory layer = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) layer[i] = leaves[i];

        while (n > 1) {
            uint256 nextN = (n + 1) / 2;
            bytes32[] memory next = new bytes32[](nextN);

            for (uint256 i = 0; i < n / 2; i++) {
                // Canonical hash: must sort layer[2*i] and layer[2*i+1]
                next[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }
            // Handle odd number of leaves (copying the last element)
            if (n % 2 == 1) {
                next[nextN - 1] = layer[n - 1];
            }

            layer = next;
            n = nextN;
        }

        return layer[0];
    }

    // FIX: Simplified Merkle proof generation to avoid complex error-prone logic for odd layers.
    // Relies on the canonical hashing of the inputs in _verifySourceCommitment.
    function getMerkleProof(bytes32[] memory leaves, uint256 index) internal pure returns (bytes32[] memory) {
        uint256 n = leaves.length;
        if (n == 0) return new bytes32[](0);
        if (n == 1) return new bytes32[](0);

        // First pass: count actual proof elements
        uint256 proofLength = 0;
        uint256 tempN = n;
        uint256 tempIndex = index;
        
        while (tempN > 1) {
            if (tempIndex % 2 == 0) {
                if (tempIndex + 1 < tempN) {
                    proofLength++;
                }
            } else {
                proofLength++;
            }
            
            tempN = (tempN + 1) / 2;
            tempIndex = tempIndex / 2;
        }

        // Second pass: build proof
        bytes32[] memory proof = new bytes32[](proofLength);
        bytes32[] memory layer = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            layer[i] = leaves[i];
        }

        uint256 proofIndex = 0;
        uint256 currentIndex = index;
        n = leaves.length;

        while (n > 1) {
            if (currentIndex % 2 == 0) {
                if (currentIndex + 1 < n) {
                    proof[proofIndex] = layer[currentIndex + 1];
                    proofIndex++;
                }
            } else {
                proof[proofIndex] = layer[currentIndex - 1];
                proofIndex++;
            }

            uint256 nextN = (n + 1) / 2;
            bytes32[] memory next = new bytes32[](nextN);
            
            for (uint256 i = 0; i < n / 2; i++) {
                next[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }
            
            if (n % 2 == 1) {
                next[nextN - 1] = layer[n - 1];
            }

            layer = next;
            n = nextN;
            currentIndex = currentIndex / 2;
        }

        return proof;
    }

    // ========== FILL INTENT TESTS ==========

    function test_FillIntent() public {
        // Single-leaf case: source root is the commitment itself, proof is empty
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.startPrank(solver);

        vm.expectEmit(true, true, false, true);
        emit IntentFilled(intentId, solver, TEST_AMOUNT);

        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        vm.stopPrank();

        // Verify fill stored
        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertEq(fill.solver, solver);
        assertEq(fill.token, address(token));
        assertEq(fill.amount, TEST_AMOUNT);
        assertEq(fill.sourceChain, SOURCE_CHAIN);
        assertFalse(fill.claimed);

        // Verify tokens transferred
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);

        // Verify merkle tree updated
        assertEq(settlement.getFillTreeSize(), 1);
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    function test_RevertWhen_FillIntent_AlreadyFilled() public {
        // Single-leaf test
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // Fill once
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        // Try to fill again
        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.AlreadyFilled.selector);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );
    }

    function test_RevertWhen_FillIntent_InvalidProof() public {
        // Sync a root that does not match the commitment or proof
        bytes32 sourceRoot = keccak256("some_other_root");
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // Invalid proof array
        bytes32[] memory invalidProof = new bytes32[](1);
        invalidProof[0] = keccak256("invalid");

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.InvalidProof.selector);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            invalidProof,
            0
        );
    }

    function test_FillIntent_MultipleSolvers() public {
        // Two commitments on the source chain → build leaves and proofs
        address solver2 = makeAddr("solver2");
        token.mint(solver2, 1000 ether);
        vm.prank(solver2);
        token.approve(address(settlement), type(uint256).max);

        // first commitment already in setUp = commitment
        bytes32 secret2 = keccak256("secret2");
        bytes32 nullifier2 = keccak256("nullifier2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier2,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        bytes32[] memory leaves = new bytes32[](2);
        leaves[0] = commitment;
        leaves[1] = commitment2;

        bytes32 sourceRoot = buildMerkleRootFromLeaves(leaves);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // First solver fills (proof for index 0)
        bytes32[] memory proof0 = getMerkleProof(leaves, 0);
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof0,
            0
        );

        // Second solver fills different intent
        bytes32 intentId2 = keccak256("intent2");

        bytes32[] memory proof1 = getMerkleProof(leaves, 1);
        vm.prank(solver2);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof1,
            1
        );

        assertEq(settlement.getFillTreeSize(), 2);
    }

    // ========== CLAIM WITHDRAWAL TESTS ==========

    function test_ClaimWithdrawal() public {
        // Single-leaf case (source root == commitment)
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        // Generate claim authorization signature
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.startPrank(relayer);

        vm.expectEmit(true, true, false, true);
        emit WithdrawalClaimed(intentId, nullifier, recipientAddr);

        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

        vm.stopPrank();

        // Verify claimed
        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertTrue(fill.claimed);
        assertTrue(settlement.isNullifierUsed(nullifier));

        // Verify tokens transferred
        uint256 fee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - fee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        assertEq(token.balanceOf(feeCollector), fee);
    }

    function test_RevertWhen_ClaimWithdrawal_Unauthorized() public {
        // Single-leaf case
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        // The actual unauthorized claim:
        vm.prank(solver); // Not relayer
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            "" // dummy
        );
    }

    function test_RevertWhen_ClaimWithdrawal_NotFilled() public {
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.NotFilled.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );
    }

    function test_RevertWhen_ClaimWithdrawal_AlreadyClaimed() public {
        // Single-leaf case
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        // Claim once
        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

        // Try to claim again
        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.AlreadyClaimed.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );
    }

    function test_RevertWhen_ClaimWithdrawal_NullifierUsed() public {
        // Single-leaf case
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        // Claim first intent
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

        // Try to use same nullifier for different intent
        bytes32 intentId2 = keccak256("intent2");
        bytes32 secret2 = keccak256("secret2");

        // Use the same nullifier!
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        // Now build a 2-leaf source tree (commitment and commitment2)
        bytes32[] memory leaves = new bytes32[](2);
        leaves[0] = commitment;
        leaves[1] = commitment2;
        bytes32 sourceRoot2 = buildMerkleRootFromLeaves(leaves);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot2);

        // Fill the second intent
        bytes32[] memory proof2 = getMerkleProof(leaves, 1);
        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot2,
            proof2,
            1
        );

        bytes32 authHash2 = keccak256(
            abi.encodePacked(intentId2, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash2 = MessageHashUtils.toEthSignedMessageHash(
            authHash2
        );
        (v, r, s) = vm.sign(recipientPrivateKey, ethSignedHash2);
        bytes memory claimAuth2 = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.NullifierUsed.selector);
        settlement.claimWithdrawal(
            intentId2,
            nullifier, // Reused nullifier
            recipientAddr,
            secret2,
            claimAuth2
        );
    }

    function test_RevertWhen_ClaimWithdrawal_InvalidCommitment() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        bytes32 wrongSecret = keccak256("wrong_secret");

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.InvalidCommitment.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            wrongSecret, // Wrong secret
            claimAuth
        );
    }

    function test_RevertWhen_ClaimWithdrawal_InvalidSignature() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        // Generate a signature using a known private key that is NOT the recipient's key (e.g., solver's derived key)
        uint256 wrongPrivateKey = 0x5678;

        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            wrongPrivateKey,
            ethSignedHash
        ); // Signed by the wrong key
        bytes memory invalidClaimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        // This should now revert because the recovered address != recipientAddr (due to Fix 2 in the contract)
        vm.expectRevert(PrivateSettlement.InvalidSignature.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            invalidClaimAuth
        );
    }

    // ========== ROOT SYNC TESTS ==========

    function test_SyncSourceChainRoot() public {
        bytes32 root = keccak256("root");

        vm.expectEmit(true, false, false, true);
        emit RootSynced(SOURCE_CHAIN, root);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, root);

        assertEq(settlement.getSourceChainRoot(SOURCE_CHAIN), root);
    }

    function test_RevertWhen_SyncSourceChainRoot_Unauthorized() public {
        bytes32 root = keccak256("root");

        vm.prank(solver); // Not relayer
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, root);
    }

    // ========== MERKLE TREE TESTS (Internal Fill Tree) ==========

    function test_GenerateFillProof_SingleFill() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        // For a single element tree, the proof should be empty.
        assertEq(fillProof.length, 0);
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    function test_GenerateFillProof_MultipleFills() public {
        // Prepare source root for first fill (single leaf)
        bytes32 sourceRoot1 = commitment;
        bytes32[] memory proof0 = new bytes32[](0);
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot1);

        // Fill 1
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot1,
            proof0,
            0
        );

        // Prepare next commitment and source root (single leaf again for simplicity)
        bytes32 intentId2 = keccak256("intent2");
        bytes32 secret2 = keccak256("secret2");
        bytes32 nullifier2 = keccak256("nullifier2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier2,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        bytes32 sourceRoot2 = commitment2;
        bytes32[] memory proof1 = new bytes32[](0);
        vm.prank(relayer);
        settlement.syncSourceChainRoot(2, sourceRoot2); // Use a different chainId for cleanliness

        // Fill 2
        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            2,
            address(token),
            TEST_AMOUNT,
            sourceRoot2,
            proof1,
            0
        );

        // Now the fillTree has 2 leaves: [intentId, intentId2]

        // 1. Check Root
        bytes32 expectedRoot = _hashPair(intentId, intentId2);
        assertEq(settlement.getMerkleRoot(), expectedRoot);

        // 2. Check Proof 1 (intentId at index 0)
        bytes32[] memory fillProof1 = settlement.generateFillProof(intentId);
        assertEq(fillProof1.length, 1);
        // The sibling is intentId2. Since _hashPair is canonical, the proof element should be intentId2.
        assertEq(fillProof1[0], intentId2);

        // 3. Check Proof 2 (intentId2 at index 1)
        bytes32[] memory fillProof2 = settlement.generateFillProof(intentId2);
        assertEq(fillProof2.length, 1);
        // The sibling is intentId.
        assertEq(fillProof2[0], intentId);
    }

    function test_MerkleRootUpdates() public {
        // Single-leaf source root syncing and empty proof
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32 root1 = settlement.getMerkleRoot();
        assertEq(root1, bytes32(0)); // Empty initially

        // Add first fill
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        bytes32 root2 = settlement.getMerkleRoot();
        assertEq(root2, intentId);

        // Add second fill (for 2-leaf tree)
        bytes32 intentId2 = keccak256("intent2_root_test");
        bytes32 secret2 = keccak256("secret2_root_test");
        bytes32 nullifier2 = keccak256("nullifier2_root_test");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier2,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        bytes32 sourceRoot2 = commitment2;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(2, sourceRoot2);

        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            2,
            address(token),
            TEST_AMOUNT,
            sourceRoot2,
            proof,
            0
        );

        bytes32 expectedRoot3 = _hashPair(intentId, intentId2);
        bytes32 root3 = settlement.getMerkleRoot();
        assertEq(root3, expectedRoot3);
    }

    // ========== INTEGRATION TESTS ==========

    function test_FullFlowWithMultipleFills() public {
        // Build 3 commitments on the source chain, then compute root and proofs
        uint256 COUNT = 3;
        bytes32[] memory leaves = new bytes32[](COUNT);
        bytes32[] memory secrets = new bytes32[](COUNT);
        bytes32[] memory nulls = new bytes32[](COUNT);
        bytes32[] memory ids = new bytes32[](COUNT);

        for (uint256 i = 0; i < COUNT; i++) {
            bytes32 s = keccak256(abi.encodePacked("secret", i));
            bytes32 n = keccak256(abi.encodePacked("nullifier", i));
            bytes32 id = keccak256(abi.encodePacked("intent", i));

            secrets[i] = s;
            nulls[i] = n;
            ids[i] = id;

            bytes32[4] memory inputs = [
                s,
                n,
                bytes32(TEST_AMOUNT),
                bytes32(uint256(SOURCE_CHAIN))
            ];
            bytes32 c = poseidon.poseidon(inputs);
            leaves[i] = c;
        }

        bytes32 sourceRoot = buildMerkleRootFromLeaves(leaves);
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        for (uint256 i = 0; i < COUNT; i++) {
            bytes32[] memory p = getMerkleProof(leaves, i);

            vm.prank(solver);
            settlement.fillIntent(
                ids[i],
                leaves[i],
                SOURCE_CHAIN,
                address(token),
                TEST_AMOUNT,
                sourceRoot,
                p,
                i
            );
        }

        assertEq(settlement.getFillTreeSize(), COUNT);
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT * COUNT);

        // Claim the middle intent (index 1)
        bytes32 targetIntent = ids[1];
        bytes32 targetNullifier = nulls[1];
        bytes32 targetSecret = secrets[1];

        bytes32 authHash = keccak256(
            abi.encodePacked(targetIntent, targetNullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        settlement.claimWithdrawal(
            targetIntent,
            targetNullifier,
            recipientAddr,
            targetSecret,
            claimAuth
        );

        // Verify claimed status
        assertTrue(settlement.getFill(targetIntent).claimed);
        assertTrue(settlement.isNullifierUsed(targetNullifier));
    }

    // ========== FUZZ TESTS ==========

    function testFuzz_FillIntent_ValidAmount(uint256 amount) public {
        amount = bound(amount, 0.001 ether, 100 ether);

        bytes32 s = keccak256("fuzz_secret");
        bytes32 n = keccak256("fuzz_nullifier");
        bytes32 id = keccak256("fuzz_intent");

        bytes32[4] memory inputs = [
            s,
            n,
            bytes32(amount),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 c = poseidon.poseidon(inputs);

        // Single-leaf on source chain
        bytes32 sourceRoot = c;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory proof = new bytes32[](0);

        token.mint(solver, amount);

        vm.prank(solver);
        settlement.fillIntent(
            id,
            c,
            SOURCE_CHAIN,
            address(token),
            amount,
            sourceRoot,
            proof,
            0
        );

        PrivateSettlement.Fill memory fill = settlement.getFill(id);
        assertEq(fill.amount, amount);
        assertEq(token.balanceOf(address(settlement)), amount);
    }

    // ========== GAS BENCHMARKS ==========

    function test_Gas_FillIntent() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.startPrank(solver);
        uint256 gasBefore = gasleft();
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for fillIntent (1-leaf Merkle):", gasUsed);
        vm.stopPrank();
    }

    function test_Gas_ClaimWithdrawal() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            proof,
            0
        );

        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            authHash
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(
            recipientPrivateKey,
            ethSignedHash
        );
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.startPrank(relayer);
        uint256 gasBefore = gasleft();
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for claimWithdrawal:", gasUsed);
        vm.stopPrank();
    }
}
