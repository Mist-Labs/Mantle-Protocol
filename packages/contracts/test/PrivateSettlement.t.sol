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

/**
 * @title PrivateSettlementTest - FIXED for Power-of-2 Merkle
 */
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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.AlreadyRegistered.selector);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);
    }

    // ========== FILL INTENT TESTS ==========

    function test_FillIntent() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    function test_RevertWhen_FillIntent_AlreadyFilled() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

    // ========== CLAIM WITHDRAWAL TESTS ==========

    function test_ClaimWithdrawal() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

    // ========== MERKLE PROOF TESTS ==========

    function test_GenerateFillProof_SingleFill() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        assertEq(fillProof.length, 0); // Single leaf needs no proof
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    function test_GenerateFillProof_MultipleFills() public {
        // Create 3 fills
        for (uint256 i = 0; i < 3; i++) {
            bytes32 s = keccak256(abi.encodePacked("secret", i));
            bytes32 n = keccak256(abi.encodePacked("nullifier", i));
            bytes32 id = keccak256(abi.encodePacked("intent", i));

            bytes32[4] memory inputs = [s, n, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN))];
            bytes32 c = poseidon.poseidon(inputs);

            bytes32 sourceRoot = c;
            bytes32[] memory proof = new bytes32[](0);

            vm.prank(relayer);
            settlement.syncSourceChainRoot(uint32(i + 1), sourceRoot);

            vm.prank(relayer);
            settlement.registerIntent(id, c, address(token), TEST_AMOUNT, uint32(i + 1), uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

            vm.prank(solver);
            settlement.fillIntent(id, c, uint32(i + 1), address(token), TEST_AMOUNT);
        }

        // Tree is padded to 4 leaves (power of 2)
        assertEq(settlement.getFillTreeSize(), 3);

        bytes32 id0 = keccak256(abi.encodePacked("intent", uint256(0)));
        bytes32[] memory fillProof = settlement.generateFillProof(id0);
        assertEq(fillProof.length, 2); // Height is 2 (4 leaves = 2^2)
    }

    // ========== GAS BENCHMARKS ==========

    function test_Gas_FillIntent() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);

        vm.prank(solver);
        uint256 gasBefore = gasleft();
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for fillIntent (fixed Merkle):", gasUsed);
    }

    function test_Gas_ClaimWithdrawal() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

        console.log("Gas used for claimWithdrawal (fixed Merkle):", gasUsed);
        vm.stopPrank();
    }
}