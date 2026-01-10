// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract MockERC20 is ERC20 {
    uint8 private _decimals;

    constructor(
        string memory name,
        string memory symbol,
        uint8 decimals_
    ) ERC20(name, symbol) {
        _decimals = decimals_;
    }

    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract IntegrationTest is Test {
    PoseidonHasher public poseidon;
    PrivateIntentPool public intentPool;
    PrivateSettlement public settlement;
    MockERC20 public token;

    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public user = makeAddr("user");
    address public solver = makeAddr("solver");
    address public owner = makeAddr("owner");

    uint256 public recipientPrivateKey = 0x1234;
    address public recipientAddr;

    uint256 public constant TEST_AMOUNT = 1 ether;
    uint32 public constant SOURCE_CHAIN = 1;
    uint32 public constant DEST_CHAIN = 1;

    function setUp() public {
        poseidon = new PoseidonHasher();
        intentPool = new PrivateIntentPool(
            owner,
            relayer,
            feeCollector,
            address(poseidon)
        );
        settlement = new PrivateSettlement(
            owner,
            relayer,
            feeCollector,
            address(poseidon)
        );
        token = new MockERC20("Mock Token", "MOCK", 18);

        recipientAddr = vm.addr(recipientPrivateKey);

        token.mint(relayer, 1000 ether);
        token.mint(solver, 1000 ether);

        vm.prank(relayer);
        token.approve(address(intentPool), type(uint256).max);

        vm.prank(solver);
        token.approve(address(settlement), type(uint256).max);

        vm.startPrank(owner);
        intentPool.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
        settlement.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
        vm.stopPrank();
    }

    function test_SingleLeafMerkleTree_ProofGeneration() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);
        vm.stopPrank();

        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent1");

        vm.prank(user);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );

        assertEq(intentPool.getCommitmentTreeSize(), 1);

        (bytes32[] memory proof, uint256 leafIndex) = intentPool
            .generateCommitmentProof(commitment);
        bytes32 root = intentPool.getMerkleRoot();

        assertEq(leafIndex, 0);
        assertEq(proof.length, 1);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, root);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            root,
            proof,
            leafIndex
        );

        assertTrue(settlement.isIntentRegistered(intentId));
    }

    function test_TwoLeafMerkleTree_ProofGeneration() public {
        token.mint(user, TEST_AMOUNT * 2);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT * 2);
        vm.stopPrank();

        bytes32[] memory commitments = new bytes32[](2);
        bytes32[] memory intentIds = new bytes32[](2);

        for (uint256 i = 0; i < 2; i++) {
            bytes32 secret = keccak256(abi.encodePacked("secret", i));
            bytes32 nullifier = keccak256(abi.encodePacked("nullifier", i));
            bytes32[4] memory inputs = [
                secret,
                nullifier,
                bytes32(TEST_AMOUNT),
                bytes32(uint256(DEST_CHAIN))
            ];
            commitments[i] = poseidon.poseidon(inputs);
            intentIds[i] = keccak256(abi.encodePacked("intent", i));

            vm.prank(user);
            intentPool.createIntent(
                intentIds[i],
                commitments[i],
                address(token),
                TEST_AMOUNT,
                address(token),
                TEST_AMOUNT - 1,
                DEST_CHAIN,
                user,
                0
            );
        }

        assertEq(intentPool.getCommitmentTreeSize(), 2);
        bytes32 root = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, root);

        for (uint256 i = 0; i < 2; i++) {
            (bytes32[] memory proof, uint256 leafIndex) = intentPool
                .generateCommitmentProof(commitments[i]);

            assertEq(leafIndex, i);
            assertEq(proof.length, 1);

            vm.prank(relayer);
            settlement.registerIntent(
                intentIds[i],
                commitments[i],
                address(token),
                TEST_AMOUNT,
                SOURCE_CHAIN,
                uint64(block.timestamp + 1 hours),
                root,
                proof,
                leafIndex
            );

            assertTrue(settlement.isIntentRegistered(intentIds[i]));
        }
    }

    function test_SingleLeafFillTree_ProofVerification() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);
        vm.stopPrank();

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

        vm.prank(user);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );

        (bytes32[] memory sourceProof, uint256 leafIndex) = intentPool
            .generateCommitmentProof(commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            sourceProof,
            leafIndex
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        assertEq(settlement.getFillTreeSize(), 1);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        bytes32 fillRoot = settlement.getMerkleRoot();

        assertEq(fillProof.length, 1);

        vm.prank(relayer);
        intentPool.syncDestChainFillRoot(DEST_CHAIN, fillRoot);

        vm.prank(relayer);
        intentPool.settleIntent(intentId, solver, fillProof, 0);

        assertTrue(intentPool.getIntent(intentId).filled);
    }

    function test_PauseContract_BlocksOperations() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);
        vm.stopPrank();

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

        vm.prank(owner);
        intentPool.pauseContract();

        assertTrue(intentPool.paused());

        vm.prank(user);
        vm.expectRevert(PrivateIntentPool.ContractIsPaused.selector);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );

        vm.prank(owner);
        intentPool.pauseContract();

        assertFalse(intentPool.paused());

        vm.prank(user);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
    }

    function test_CancelIntent_RefundsUser() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );

        uint256 balanceBefore = token.balanceOf(user);
        intentPool.cancelIntent(intentId);
        uint256 balanceAfter = token.balanceOf(user);

        assertEq(balanceAfter - balanceBefore, TEST_AMOUNT);
        assertTrue(intentPool.getIntent(intentId).refunded);
        vm.stopPrank();
    }

    function test_UserClaimRefund_AfterDeadline() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        vm.warp(block.timestamp + 2 hours + 300);

        uint256 balanceBefore = token.balanceOf(user);
        vm.prank(user);
        intentPool.userClaimRefund(intentId);
        uint256 balanceAfter = token.balanceOf(user);

        assertEq(balanceAfter - balanceBefore, TEST_AMOUNT);
        assertTrue(intentPool.getIntent(intentId).refunded);
    }

    function test_RevertWhen_UserClaimRefundBeforeBuffer() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        vm.warp(block.timestamp + 2 hours + 100);

        vm.prank(user);
        vm.expectRevert(PrivateIntentPool.BufferPeriodActive.selector);
        intentPool.userClaimRefund(intentId);
    }

    function test_UpdateRelayer() public {
        address newRelayer = makeAddr("newRelayer");

        vm.prank(owner);
        intentPool.updateRelayer(newRelayer);

        assertEq(intentPool.RELAYER(), newRelayer);

        vm.prank(owner);
        settlement.updateRelayer(newRelayer);

        assertEq(settlement.RELAYER(), newRelayer);
    }

    function test_UpdatePoseidonHasher() public {
        PoseidonHasher newPoseidon = new PoseidonHasher();

        vm.prank(owner);
        intentPool.updatePoseidonHasher(address(newPoseidon));

        assertEq(address(intentPool.POSEIDON_HASHER()), address(newPoseidon));

        vm.prank(owner);
        settlement.updatePoseidonHasher(address(newPoseidon));

        assertEq(address(settlement.POSEIDON_HASHER()), address(newPoseidon));
    }

    function test_EmergencyWithdraw() public {
        token.mint(address(intentPool), TEST_AMOUNT);

        vm.prank(owner);
        intentPool.pauseContract();

        vm.warp(block.timestamp + 30 days + 1);

        uint256 feeCollectorBalanceBefore = token.balanceOf(feeCollector);

        vm.prank(owner);
        intentPool.emergencyWithdraw(address(token), TEST_AMOUNT);

        uint256 feeCollectorBalanceAfter = token.balanceOf(feeCollector);

        assertEq(
            feeCollectorBalanceAfter - feeCollectorBalanceBefore,
            TEST_AMOUNT
        );
    }

    function test_RevertWhen_EmergencyWithdrawBeforeDelay() public {
        token.mint(address(intentPool), TEST_AMOUNT);

        vm.prank(owner);
        intentPool.pauseContract();

        vm.warp(block.timestamp + 15 days);

        vm.prank(owner);
        vm.expectRevert(PrivateIntentPool.EmergencyPeriodNotReached.selector);
        intentPool.emergencyWithdraw(address(token), TEST_AMOUNT);
    }

    function test_RelayerRefund_AfterDeadline() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        vm.warp(block.timestamp + 2 hours + 1);

        uint256 userBalanceBefore = token.balanceOf(user);

        vm.prank(relayer);
        intentPool.refund(intentId);

        uint256 userBalanceAfter = token.balanceOf(user);

        assertEq(userBalanceAfter - userBalanceBefore, TEST_AMOUNT);
        assertTrue(intentPool.getIntent(intentId).refunded);
    }

    function test_RevertWhen_NonRelayerCallsRefund() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        vm.warp(block.timestamp + 2 hours + 1);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(PrivateIntentPool.Unauthorized.selector);
        intentPool.refund(intentId);
    }

    function test_FullFlow_CreateFillClaim() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);
        vm.stopPrank();

        bytes32 secret = keccak256("user_secret");
        bytes32 nullifier = keccak256("user_nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256(
            abi.encodePacked(block.timestamp, "intent1")
        );

        vm.prank(user);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );

        assertEq(token.balanceOf(address(intentPool)), TEST_AMOUNT);

        (bytes32[] memory sourceProof, uint256 leafIndex) = intentPool
            .generateCommitmentProof(commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            sourceProof,
            leafIndex
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);

        bytes32[] memory destProof = settlement.generateFillProof(intentId);
        bytes32 destRoot = settlement.getMerkleRoot();

        vm.prank(relayer);
        intentPool.syncDestChainFillRoot(DEST_CHAIN, destRoot);

        vm.prank(relayer);
        intentPool.settleIntent(intentId, solver, destProof, 0);

        uint256 poolFee = (TEST_AMOUNT * intentPool.FEE_BPS()) / 10000;
        assertEq(intentPool.getSolver(intentId), solver);

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

        uint256 settlementFee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - settlementFee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        assertEq(token.balanceOf(feeCollector), poolFee + settlementFee);
    }

    function test_FullFlow_MultipleIntents() public {
        token.mint(user, TEST_AMOUNT * 3);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT * 3);
        vm.stopPrank();

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

            vm.prank(user);
            intentPool.createIntent(
                intentId,
                commitment,
                address(token),
                TEST_AMOUNT,
                address(token),
                TEST_AMOUNT - 1,
                DEST_CHAIN,
                user,
                0
            );

            (bytes32[] memory proof, uint256 leafIndex) = intentPool
                .generateCommitmentProof(commitment);
            bytes32 sourceRoot = intentPool.getMerkleRoot();

            vm.prank(relayer);
            settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

            vm.prank(relayer);
            settlement.registerIntent(
                intentId,
                commitment,
                address(token),
                TEST_AMOUNT,
                SOURCE_CHAIN,
                uint64(block.timestamp + 1 hours),
                sourceRoot,
                proof,
                leafIndex
            );

            vm.prank(solver);
            settlement.fillIntent(
                intentId,
                commitment,
                SOURCE_CHAIN,
                address(token),
                TEST_AMOUNT
            );
        }

        assertEq(settlement.getFillTreeSize(), 3);
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT * 3);
    }

    function test_RevertWhen_NonRelayerCallsSettleIntent() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

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

        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        intentPool.syncDestChainFillRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(PrivateIntentPool.Unauthorized.selector);
        intentPool.settleIntent(intentId, solver, proof, 0);
    }

    function test_RevertWhen_NonRelayerRegistersIntent() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            proof,
            0
        );
    }

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

        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

        uint256 gasStart = gasleft();
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            address(token),
            TEST_AMOUNT - 1,
            DEST_CHAIN,
            user,
            0
        );
        vm.stopPrank();

        uint256 gasAfterCreate = gasleft();
        console.log("Gas for createIntent:", gasStart - gasAfterCreate);

        (bytes32[] memory sourceProof, uint256 leafIndex) = intentPool
            .generateCommitmentProof(commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            sourceProof,
            leafIndex
        );

        gasStart = gasleft();
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );
        uint256 gasAfterFill = gasleft();
        console.log("Gas for fillIntent:", gasStart - gasAfterFill);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainFillRoot(DEST_CHAIN, destRoot);

        gasStart = gasleft();
        vm.prank(relayer);
        intentPool.settleIntent(intentId, solver, fillProof, 0);
        console.log("Gas for settleIntent:", gasStart - gasleft());
    }
}
