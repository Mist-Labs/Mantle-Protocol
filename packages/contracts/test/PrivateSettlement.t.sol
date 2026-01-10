// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract MockERC20 is ERC20 {
    uint8 private _decimals;

    constructor() ERC20("Mock Token", "MOCK") {
        _decimals = 18;
    }

    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract PrivateSettlementTest is Test {
    PoseidonHasher public poseidon;
    PrivateSettlement public settlement;
    MockERC20 public token;

    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public solver = makeAddr("solver");
    address public recipient = makeAddr("recipient");
    address public owner = makeAddr("owner");

    uint256 public recipientPrivateKey = 0x1234;
    address public recipientAddr;

    bytes32 public secret;
    bytes32 public nullifier;
    bytes32 public commitment;
    bytes32 public intentId;

    uint256 public constant TEST_AMOUNT = 1 ether;
    uint32 public constant SOURCE_CHAIN = 1;

    event IntentFilled(bytes32 indexed intentId, address indexed solver, address indexed token, uint256 amount);
    event WithdrawalClaimed(bytes32 indexed intentId, bytes32 indexed nullifier, address token);
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event MerkleRootUpdated(bytes32 root);
    event TokenAdded(address indexed token, uint256 minAmount, uint256 maxAmount);
    event TokenRemoved(address indexed token);
    event ContractPaused(bool paused);
    event RelayerUpdated(address indexed oldRelayer, address indexed newRelayer);
    event PoseidonHasherUpdated(address indexed oldHasher, address indexed newHasher);
    event EmergencyWithdrawal(address indexed token, uint256 amount, address indexed recipient);

    function setUp() public {
        poseidon = new PoseidonHasher();
        settlement = new PrivateSettlement(owner, relayer, feeCollector, address(poseidon));
        token = new MockERC20();

        recipientAddr = vm.addr(recipientPrivateKey);

        secret = keccak256("secret");
        nullifier = keccak256("nullifier");
        intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN))];
        commitment = poseidon.poseidon(inputs);

        token.mint(solver, 1000 ether);
        vm.prank(solver);
        token.approve(address(settlement), type(uint256).max);

        vm.prank(owner);
        settlement.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return a < b ? keccak256(abi.encodePacked(a, b)) : keccak256(abi.encodePacked(b, a));
    }

    function _computeSingleLeafRoot(bytes32 leaf) internal pure returns (bytes32) {
        return _hashPair(leaf, bytes32(0));
    }

    // ========== TOKEN WHITELIST TESTS ==========

    function test_AddSupportedToken() public {
        MockERC20 newToken = new MockERC20();

        vm.expectEmit(true, false, false, true);
        emit TokenAdded(address(newToken), 0.01 ether, 100 ether);

        vm.prank(owner);
        settlement.addSupportedToken(address(newToken), 0.01 ether, 100 ether, 18);

        assertTrue(settlement.isTokenSupported(address(newToken)));
        assertEq(settlement.getSupportedTokenCount(), 2);

        PrivateSettlement.TokenConfig memory config = settlement.getTokenConfig(address(newToken));
        assertEq(config.minFillAmount, 0.01 ether);
        assertEq(config.maxFillAmount, 100 ether);
        assertEq(config.decimals, 18);
    }

    function test_RemoveSupportedToken() public {
        vm.expectEmit(true, false, false, false);
        emit TokenRemoved(address(token));

        vm.prank(owner);
        settlement.removeSupportedToken(address(token));

        assertFalse(settlement.isTokenSupported(address(token)));
        assertEq(settlement.getSupportedTokenCount(), 0);
    }

    // ========== REGISTER INTENT TESTS ==========

    function test_RegisterIntent() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        assertTrue(settlement.isIntentRegistered(intentId));

        PrivateSettlement.IntentParams memory params = settlement.getIntentParams(intentId);
        assertEq(params.commitment, commitment);
        assertEq(params.token, address(token));
        assertEq(params.amount, TEST_AMOUNT);
        assertTrue(params.exists);
    }

    function test_RevertWhen_RegisterIntent_AlreadyRegistered() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.AlreadyRegistered.selector);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);
    }

    function test_RevertWhen_RegisterIntent_Paused() public {
        vm.prank(owner);
        settlement.pauseContract();

        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.ContractIsPaused.selector);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);
    }

    // ========== FILL INTENT TESTS ==========

    function test_FillIntent() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.expectEmit(true, true, false, true);
        emit IntentFilled(intentId, solver, address(token), TEST_AMOUNT);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertEq(fill.solver, solver);
        assertEq(fill.token, address(token));
        assertEq(fill.amount, TEST_AMOUNT);
        assertFalse(fill.claimed);

        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);
        assertEq(settlement.getFillTreeSize(), 1);
        
        bytes32 expectedRoot = _computeSingleLeafRoot(intentId);
        assertEq(settlement.getMerkleRoot(), expectedRoot);
    }

    function test_RevertWhen_FillIntent_AlreadyFilled() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.AlreadyFilled.selector);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
    }

    function test_RevertWhen_FillIntent_NotRegistered() public {
        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.IntentNotRegistered.selector);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
    }

    function test_RevertWhen_FillIntent_Paused() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(owner);
        settlement.pauseContract();

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.ContractIsPaused.selector);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
    }

    // ========== CLAIM WITHDRAWAL TESTS ==========

    function test_ClaimWithdrawal() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.startPrank(relayer);
        vm.expectEmit(true, true, false, true);
        emit WithdrawalClaimed(intentId, nullifier, address(token));

        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);
        vm.stopPrank();

        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertTrue(fill.claimed);
        assertTrue(settlement.isNullifierUsed(nullifier));

        uint256 fee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - fee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        assertEq(token.balanceOf(feeCollector), fee);
    }

    function test_ClaimWithdrawal_WorksWhenPaused() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        vm.prank(owner);
        settlement.pauseContract();

        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);

        assertTrue(settlement.getFill(intentId).claimed);
    }

    function test_RevertWhen_ClaimWithdrawal_NotFilled() public {
        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.NotFilled.selector);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);
    }

    function test_RevertWhen_ClaimWithdrawal_AlreadyClaimed() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.AlreadyClaimed.selector);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);
    }

    function test_RevertWhen_ClaimWithdrawal_InvalidCommitment() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        bytes32 wrongSecret = keccak256("wrong_secret");

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.InvalidCommitment.selector);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, wrongSecret, claimAuth);
    }

    // ========== PAUSE TESTS ==========

    function test_PauseContract() public {
        assertFalse(settlement.paused());

        vm.expectEmit(false, false, false, true);
        emit ContractPaused(true);

        vm.prank(owner);
        settlement.pauseContract();

        assertTrue(settlement.paused());
        assertTrue(settlement.pausedAt() > 0);
    }

    function test_UnpauseContract() public {
        vm.prank(owner);
        settlement.pauseContract();
        assertTrue(settlement.paused());

        vm.prank(owner);
        settlement.pauseContract();
        assertFalse(settlement.paused());
    }

    // ========== UPDATE FUNCTIONS TESTS ==========

    function test_UpdateRelayer() public {
        address newRelayer = makeAddr("newRelayer");

        vm.expectEmit(true, true, false, false);
        emit RelayerUpdated(relayer, newRelayer);

        vm.prank(owner);
        settlement.updateRelayer(newRelayer);

        assertEq(settlement.RELAYER(), newRelayer);
    }

    function test_RevertWhen_UpdateRelayer_NotOwner() public {
        address newRelayer = makeAddr("newRelayer");

        vm.prank(solver);
        vm.expectRevert();
        settlement.updateRelayer(newRelayer);
    }

    function test_UpdatePoseidonHasher() public {
        PoseidonHasher newPoseidon = new PoseidonHasher();

        vm.expectEmit(true, true, false, false);
        emit PoseidonHasherUpdated(address(poseidon), address(newPoseidon));

        vm.prank(owner);
        settlement.updatePoseidonHasher(address(newPoseidon));

        assertEq(address(settlement.POSEIDON_HASHER()), address(newPoseidon));
    }

    // ========== EMERGENCY WITHDRAW TESTS ==========

    function test_EmergencyWithdraw() public {
        token.mint(address(settlement), TEST_AMOUNT);

        vm.prank(owner);
        settlement.pauseContract();

        vm.warp(block.timestamp + settlement.EMERGENCY_WITHDRAW_DELAY() + 1);

        uint256 feeCollectorBalanceBefore = token.balanceOf(feeCollector);

        vm.expectEmit(true, false, true, true);
        emit EmergencyWithdrawal(address(token), TEST_AMOUNT, feeCollector);

        vm.prank(owner);
        settlement.emergencyWithdraw(address(token), TEST_AMOUNT);

        uint256 feeCollectorBalanceAfter = token.balanceOf(feeCollector);
        assertEq(feeCollectorBalanceAfter - feeCollectorBalanceBefore, TEST_AMOUNT);
    }

    function test_RevertWhen_EmergencyWithdraw_NotPaused() public {
        token.mint(address(settlement), TEST_AMOUNT);

        vm.prank(owner);
        vm.expectRevert(PrivateSettlement.ContractNotPaused.selector);
        settlement.emergencyWithdraw(address(token), TEST_AMOUNT);
    }

    function test_RevertWhen_EmergencyWithdraw_BeforeDelay() public {
        token.mint(address(settlement), TEST_AMOUNT);

        vm.prank(owner);
        settlement.pauseContract();

        vm.warp(block.timestamp + 15 days);

        vm.prank(owner);
        vm.expectRevert(PrivateSettlement.EmergencyPeriodNotReached.selector);
        settlement.emergencyWithdraw(address(token), TEST_AMOUNT);
    }

    // ========== MERKLE PROOF TESTS ==========

    function test_GenerateFillProof_SingleFill() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        assertEq(fillProof.length, 1);
        
        bytes32 root = settlement.getMerkleRoot();
        bytes32 expectedRoot = _computeSingleLeafRoot(intentId);
        assertEq(root, expectedRoot);
        assertTrue(root != intentId);
    }

    function test_GenerateFillProof_TwoFills() public {
        bytes32 s1 = keccak256("secret1");
        bytes32 n1 = keccak256("nullifier1");
        bytes32 id1 = keccak256("intent1");
        bytes32[4] memory inputs1 = [s1, n1, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN))];
        bytes32 c1 = poseidon.poseidon(inputs1);

        bytes32 sourceRoot1 = _computeSingleLeafRoot(c1);
        bytes32[] memory proof1 = new bytes32[](1);
        proof1[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot1);

        vm.prank(relayer);
        settlement.registerIntent(id1, c1, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot1, proof1, 0);

        vm.prank(solver);
        settlement.fillIntent(id1, c1, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32 s2 = keccak256("secret2");
        bytes32 n2 = keccak256("nullifier2");
        bytes32 id2 = keccak256("intent2");
        bytes32[4] memory inputs2 = [s2, n2, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN + 1))];
        bytes32 c2 = poseidon.poseidon(inputs2);

        bytes32 sourceRoot2 = _computeSingleLeafRoot(c2);
        bytes32[] memory proof2 = new bytes32[](1);
        proof2[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN + 1, sourceRoot2);

        vm.prank(relayer);
        settlement.registerIntent(id2, c2, address(token), TEST_AMOUNT, SOURCE_CHAIN + 1, uint64(block.timestamp + 1 hours), sourceRoot2, proof2, 0);

        vm.prank(solver);
        settlement.fillIntent(id2, c2, SOURCE_CHAIN + 1, address(token), TEST_AMOUNT);

        assertEq(settlement.getFillTreeSize(), 2);

        bytes32[] memory fillProof1 = settlement.generateFillProof(id1);
        assertEq(fillProof1.length, 1);
        assertEq(fillProof1[0], id2);

        bytes32[] memory fillProof2 = settlement.generateFillProof(id2);
        assertEq(fillProof2.length, 1);
        assertEq(fillProof2[0], id1);
    }

    function test_GenerateFillProof_MultipleFills() public {
        for (uint256 i = 0; i < 3; i++) {
            bytes32 s = keccak256(abi.encodePacked("secret", i));
            bytes32 n = keccak256(abi.encodePacked("nullifier", i));
            bytes32 id = keccak256(abi.encodePacked("intent", i));

            bytes32[4] memory inputs = [s, n, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN))];
            bytes32 c = poseidon.poseidon(inputs);

            bytes32 sourceRoot = _computeSingleLeafRoot(c);
            bytes32[] memory proof = new bytes32[](1);
            proof[0] = bytes32(0);

            vm.prank(relayer);
            settlement.syncSourceChainCommitmentRoot(uint32(i + 1), sourceRoot);

            vm.prank(relayer);
            settlement.registerIntent(id, c, address(token), TEST_AMOUNT, uint32(i + 1), uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

            vm.prank(solver);
            settlement.fillIntent(id, c, uint32(i + 1), address(token), TEST_AMOUNT);
        }

        assertEq(settlement.getFillTreeSize(), 3);

        bytes32 id0 = keccak256(abi.encodePacked("intent", uint256(0)));
        bytes32[] memory fillProof = settlement.generateFillProof(id0);
        assertEq(fillProof.length, 2);
    }

    // ========== GAS BENCHMARKS ==========

    function test_Gas_FillIntent() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        uint256 gasBefore = gasleft();
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for fillIntent:", gasUsed);
    }

    function test_Gas_ClaimWithdrawal() public {
        bytes32 sourceRoot = _computeSingleLeafRoot(commitment);
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = bytes32(0);

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.startPrank(relayer);
        uint256 gasBefore = gasleft();
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for claimWithdrawal:", gasUsed);
        vm.stopPrank();
    }
}