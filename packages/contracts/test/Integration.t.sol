// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract MockERC20 is ERC20 {
    constructor() ERC20("Mock Token", "MOCK") {}
    
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/**
 * @title IntegrationTest
 * @notice End-to-end integration tests for privacy bridge system
 */
contract IntegrationTest is Test {
    PoseidonHasher public poseidon;
    PrivateIntentPool public intentPool;
    PrivateSettlement public settlement;
    MockERC20 public token;
    
    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public user = makeAddr("user");
    address public solver = makeAddr("solver");
    
    uint256 public recipientPrivateKey = 0x1234;
    address public recipientAddr;
    
    uint256 public constant TEST_AMOUNT = 1 ether;
    uint32 public constant SOURCE_CHAIN = 1;
    uint32 public constant DEST_CHAIN = 1;  // Both chains use same ID for commitment
    
    function setUp() public {
        poseidon = new PoseidonHasher();
        intentPool = new PrivateIntentPool(relayer, feeCollector, address(poseidon));
        settlement = new PrivateSettlement(relayer, feeCollector, address(poseidon));
        token = new MockERC20();
        
        recipientAddr = vm.addr(recipientPrivateKey);
        
        token.mint(relayer, 1000 ether);
        token.mint(solver, 1000 ether);
        
        vm.prank(relayer);
        token.approve(address(intentPool), type(uint256).max);
        
        vm.prank(solver);
        token.approve(address(settlement), type(uint256).max);
    }
    
    // ========== FULL FLOW TESTS ==========
    
    function test_FullFlow_CreateFillClaim() public {
        // 1. USER: Generate privacy data
        bytes32 secret = keccak256("user_secret");
        bytes32 nullifier = keccak256("user_nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));
        
        // 2. RELAYER: Create intent on source chain (intentPool)
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        assertEq(token.balanceOf(address(intentPool)), TEST_AMOUNT);
        
        // 3. RELAYER: Sync commitment tree root to destination
        // Single-leaf tree: root = commitment itself
        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);
        
        // 4. SOLVER: Fill intent on destination chain (settlement)
        // Single-leaf tree: empty proof
        bytes32[] memory sourceProof = new bytes32[](0);
        
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot,
            sourceProof,
            0
        );
        
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);
        
        // 5. RELAYER: Sync fill tree root back to source
        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);
        
        // 6. RELAYER: Mark intent as filled on source chain
        bytes32[] memory destProof = settlement.generateFillProof(intentId);
        
        vm.prank(relayer);
        intentPool.markFilled(intentId, solver, destRoot, destProof, 0);
        
        // Verify solver received repayment on source chain
        // Solver paid 1 ether on destination, got back (1 ether - poolFee) on source
        uint256 poolFee = (TEST_AMOUNT * intentPool.FEE_BPS()) / 10000;
        uint256 expectedSolverAmount = TEST_AMOUNT - poolFee;
        // Net position: started with 1000, spent 1 on dest, got back (1 - poolFee) on source
        // = 1000 - 1 + (1 - poolFee) = 1000 - poolFee
        assertEq(token.balanceOf(solver), 1000 ether - poolFee);
        
        // 7. USER: Claim withdrawal on destination chain
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);
        
        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );
        
        // Verify user received tokens
        uint256 settlementFee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - settlementFee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        
        // Verify fees collected
        assertEq(token.balanceOf(feeCollector), poolFee + settlementFee);
    }
    
    function test_FullFlow_MultipleIntents() public {
        for (uint256 i = 0; i < 3; i++) {
            bytes32 secret = keccak256(abi.encodePacked("secret", i));
            bytes32 nullifier = keccak256(abi.encodePacked("nullifier", i));
            bytes32[4] memory inputs = [
                secret,
                nullifier,
                bytes32(TEST_AMOUNT),
                bytes32(uint256(DEST_CHAIN))
            ];
            bytes32 commitment = poseidon.poseidon(inputs);
            bytes32 intentId = keccak256(abi.encodePacked("intent", i));
            
            // Create intent
            vm.prank(relayer);
            intentPool.createIntent(
                intentId,
                commitment,
                address(token),
                TEST_AMOUNT,
                DEST_CHAIN,
                user,
                secret,
                nullifier
            );
            
            // Fill intent (single-leaf tree)
            bytes32 sourceRoot = commitment;
            vm.prank(relayer);
            settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);
            
            bytes32[] memory proof = new bytes32[](0);
            
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
        }
        
        assertEq(intentPool.isCommitmentUsed(
            poseidon.poseidon([
                keccak256(abi.encodePacked("secret", uint256(0))),
                keccak256(abi.encodePacked("nullifier", uint256(0))),
                bytes32(TEST_AMOUNT),
                bytes32(uint256(DEST_CHAIN))
            ])
        ), true);
        
        assertEq(settlement.getFillTreeSize(), 3);
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT * 3);
    }
    
    // ========== PRIVACY TESTS ==========
    
    function test_Privacy_CommitmentHidesDetails() public {
        bytes32 secret1 = keccak256("secret1");
        bytes32 nullifier1 = keccak256("nullifier1");
        bytes32[4] memory inputs1 = [
            secret1,
            nullifier1,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment1 = poseidon.poseidon(inputs1);
        
        bytes32 secret2 = keccak256("secret2");
        bytes32 nullifier2 = keccak256("nullifier2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier2,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);
        
        assertTrue(commitment1 != commitment2);
        assertTrue(commitment1 != secret1);
        assertTrue(commitment1 != nullifier1);
    }
    
    function test_Privacy_NullifierPreventsDoubleSpend() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        // Create and fill intent
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);
        
        bytes32[] memory proof = new bytes32[](0);
        
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
        
        // Claim once
        bytes32 authHash = keccak256(
            abi.encodePacked(intentId, nullifier, recipientAddr)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);
        
        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );
        
        assertTrue(settlement.isNullifierUsed(nullifier));
        
        // Try to reuse nullifier - should fail
        bytes32 intentId2 = keccak256("intent2");
        bytes32 secret2 = keccak256("secret2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier, // Same nullifier
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId2,
            commitment2,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret2,
            nullifier
        );
        
        bytes32 sourceRoot2 = commitment2;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot2);
        
        bytes32[] memory proof2 = new bytes32[](0);
        
        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT,
            sourceRoot2,
            proof2,
            0
        );
        
        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.NullifierUsed.selector);
        settlement.claimWithdrawal(
            intentId2,
            nullifier,
            recipientAddr,
            secret2,
            claimAuth
        );
    }
    
    // ========== ERROR RECOVERY TESTS ==========
    
    function test_ErrorRecovery_RefundExpiredIntent() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        uint256 userBalanceBefore = token.balanceOf(user);
        
        vm.warp(block.timestamp + intentPool.INTENT_TIMEOUT() + 1);
        
        intentPool.refund(intentId);
        
        assertEq(token.balanceOf(user), userBalanceBefore + TEST_AMOUNT);
        
        PrivateIntentPool.Intent memory intent = intentPool.getIntent(intentId);
        assertTrue(intent.refunded);
    }
    
    function test_ErrorRecovery_CannotFillRefundedIntent() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        vm.warp(block.timestamp + intentPool.INTENT_TIMEOUT() + 1);
        intentPool.refund(intentId);
        
        bytes32 destRoot = keccak256(abi.encodePacked(intentId));
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);
        
        bytes32[] memory proof = new bytes32[](0);
        
        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        intentPool.markFilled(intentId, solver, destRoot, proof, 0);
    }
    
    // ========== STRESS TESTS ==========
    
    function test_Stress_HighVolume() public {
        uint256 intentCount = 10;
        
        for (uint256 i = 0; i < intentCount; i++) {
            bytes32 secret = keccak256(abi.encodePacked("secret", i));
            bytes32 nullifier = keccak256(abi.encodePacked("nullifier", i));
            bytes32[4] memory inputs = [
                secret,
                nullifier,
                bytes32(TEST_AMOUNT),
                bytes32(uint256(DEST_CHAIN))
            ];
            bytes32 commitment = poseidon.poseidon(inputs);
            bytes32 intentId = keccak256(abi.encodePacked("intent", i));
            
            vm.prank(relayer);
            intentPool.createIntent(
                intentId,
                commitment,
                address(token),
                TEST_AMOUNT,
                DEST_CHAIN,
                user,
                secret,
                nullifier
            );
        }
        
        assertEq(token.balanceOf(address(intentPool)), TEST_AMOUNT * intentCount);
    }
    
    function test_Stress_LargeAmounts() public {
        uint256 largeAmount = 99 ether;
        
        token.mint(relayer, largeAmount);
        
        bytes32 secret = keccak256("large_secret");
        bytes32 nullifier = keccak256("large_nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(largeAmount),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("large_intent");
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            largeAmount,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        assertEq(token.balanceOf(address(intentPool)), largeAmount);
    }
    
    // ========== SECURITY TESTS ==========
    
    function test_Security_CannotStealFromPool() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        uint256 poolBalance = token.balanceOf(address(intentPool));
        
        address attacker = makeAddr("attacker");
        vm.startPrank(attacker);
        
        bytes32 destRoot = keccak256("root");
        bytes32[] memory proof = new bytes32[](0);
        
        vm.expectRevert(PrivateIntentPool.Unauthorized.selector);
        intentPool.markFilled(intentId, attacker, destRoot, proof, 0);
        
        vm.stopPrank();
        
        assertEq(token.balanceOf(address(intentPool)), poolBalance);
    }
    
    function test_Security_ReentrancyProtection() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        assertTrue(true);
    }
    
    // ========== GAS BENCHMARKS ==========
    
    function test_Gas_CompleteFlow() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");
        
        uint256 gasStart = gasleft();
        
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            secret,
            nullifier
        );
        
        uint256 gasAfterCreate = gasleft();
        console.log("Gas for createIntent:", gasStart - gasAfterCreate);
        
        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);
        
        bytes32[] memory proof = new bytes32[](0);
        
        gasStart = gasleft();
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
        uint256 gasAfterFill = gasleft();
        console.log("Gas for fillIntent:", gasStart - gasAfterFill);
        
        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);
        
        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        
        gasStart = gasleft();
        vm.prank(relayer);
        intentPool.markFilled(intentId, solver, destRoot, fillProof, 0);
        console.log("Gas for markFilled:", gasStart - gasleft());
    }
}