// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

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
 * @title PrivateIntentPoolTest - FIXED for Power-of-2 Merkle
 */
contract PrivateIntentPoolTest is Test {
    PoseidonHasher public poseidon;
    PrivateIntentPool public pool;
    MockERC20 public token;

    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public user = makeAddr("user");
    address public solver = makeAddr("solver");
    address public owner = makeAddr("owner");

    bytes32 public secret;
    bytes32 public nullifier;
    bytes32 public commitment;
    bytes32 public intentId;

    uint256 public constant TEST_AMOUNT = 1 ether;
    uint32 public constant DEST_CHAIN = 1;

    event IntentCreated(bytes32 indexed intentId, bytes32 indexed commitment, uint32 destChain, address sourceToken, uint256 sourceAmount, address destToken, uint256 destAmount);
    event IntentSettled(bytes32 indexed intentId, address indexed solver, bytes32 fillRoot);
    event IntentRefunded(bytes32 indexed intentId, uint256 amount);
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event MerkleRootUpdated(bytes32 root);
    event TokenAdded(address indexed token, uint256 minAmount, uint256 maxAmount);
    event TokenRemoved(address indexed token);

    function setUp() public {
        poseidon = new PoseidonHasher();
        pool = new PrivateIntentPool(owner, relayer, feeCollector, address(poseidon));
        token = new MockERC20();

        secret = keccak256("secret");
        nullifier = keccak256("nullifier");
        intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
        commitment = poseidon.poseidon(inputs);

        token.mint(relayer, 1000 ether);
        vm.prank(relayer);
        token.approve(address(pool), type(uint256).max);

        vm.prank(owner);
        pool.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    // ========== TOKEN WHITELIST TESTS ==========

    function test_AddSupportedToken() public {
        MockERC20 newToken = new MockERC20();

        vm.expectEmit(true, false, false, false);
        emit TokenAdded(address(newToken), 0.01 ether, 100 ether);

        vm.prank(owner);
        pool.addSupportedToken(address(newToken), 0.01 ether, 100 ether, 18);

        assertTrue(pool.isTokenSupported(address(newToken)));

        address[] memory list = pool.getSupportedTokens();
        assertEq(list.length, 2);
        assertEq(list[1], address(newToken));

        assertEq(pool.getSupportedTokenCount(), 2);

        PrivateIntentPool.TokenConfig memory config = pool.getTokenConfig(address(newToken));
        assertEq(config.minFillAmount, 0.01 ether);
        assertEq(config.maxFillAmount, 100 ether);
        assertEq(config.decimals, 18);
    }

    function test_RevertWhen_AddSupportedToken_NotOwner() public {
        MockERC20 newToken = new MockERC20();

        vm.prank(user);
        vm.expectRevert();
        pool.addSupportedToken(address(newToken), 0.01 ether, 100 ether, 18);
    }

    function test_RemoveSupportedToken() public {
        vm.expectEmit(true, false, false, false);
        emit TokenRemoved(address(token));

        vm.prank(owner);
        pool.removeSupportedToken(address(token));

        assertFalse(pool.isTokenSupported(address(token)));
        assertEq(pool.getSupportedTokenCount(), 0);
    }

    function test_UpdateTokenConfig() public {
        vm.prank(owner);
        pool.updateTokenConfig(address(token), 0.1 ether, 50 ether);

        PrivateIntentPool.TokenConfig memory config = pool.getTokenConfig(address(token));
        assertEq(config.minFillAmount, 0.1 ether);
        assertEq(config.maxFillAmount, 50 ether);
    }

    // ========== INTENT CREATION TESTS ==========

    function test_CreateIntent() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);

        vm.expectEmit(true, true, false, true);
        emit IntentCreated(intentId, commitment, DEST_CHAIN, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1);

        vm.expectEmit(false, false, false, true);
        emit MerkleRootUpdated(commitment);

        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertEq(intent.commitment, commitment);
        assertEq(intent.sourceToken, address(token));
        assertEq(intent.sourceAmount, TEST_AMOUNT);
        assertFalse(intent.filled);
        assertFalse(intent.refunded);

        assertTrue(pool.isCommitmentUsed(commitment));
        assertEq(token.balanceOf(address(pool)), TEST_AMOUNT);
    }

    function test_RevertWhen_CreateIntent_TokenNotSupported() public {
        MockERC20 unsupportedToken = new MockERC20();
        unsupportedToken.mint(user, 1000 ether);

        vm.startPrank(user);
        unsupportedToken.approve(address(pool), type(uint256).max);

        vm.expectRevert(PrivateIntentPool.TokenNotSupported.selector);
        pool.createIntent(intentId, commitment, address(unsupportedToken), TEST_AMOUNT, address(unsupportedToken), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();
    }

    function test_RevertWhen_CreateIntent_DuplicateCommitment() public {
        token.mint(user, TEST_AMOUNT * 2);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT * 2);

        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);

        bytes32 intentId2 = keccak256("intent2");
        vm.expectRevert(PrivateIntentPool.DuplicateCommitment.selector);
        pool.createIntent(intentId2, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();
    }

    // ========== SETTLE INTENT TESTS ==========

    function test_SettleIntent_WithProof() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        // In real flow, destination chain has a fillTree containing intentIds
        // For this test, we simulate that the intentId is the only leaf in dest tree
        bytes32 destRoot = intentId; // Single-leaf tree where root = leaf
        bytes32[] memory proof = new bytes32[](0); // No proof needed for single leaf

        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        vm.prank(relayer);
        vm.expectEmit(true, true, false, false);
        emit IntentSettled(intentId, solver, destRoot);
        
        pool.settleIntent(intentId, solver, proof, 0);

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertTrue(intent.filled);
        assertEq(pool.getSolver(intentId), solver);

        uint256 fee = (TEST_AMOUNT * pool.FEE_BPS()) / 10000;
        uint256 expectedSolverAmount = TEST_AMOUNT - fee;
        assertEq(token.balanceOf(solver), expectedSolverAmount);
        assertEq(token.balanceOf(feeCollector), fee);
    }

    function test_RevertWhen_SettleIntent_RootNotSynced() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.RootNotSynced.selector);
        pool.settleIntent(intentId, solver, proof, 0);
    }

    function test_RevertWhen_SettleIntent_AlreadySettled() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        pool.settleIntent(intentId, solver, proof, 0);

        address solver2 = makeAddr("solver2");
        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.IntentAlreadySettled.selector);
        pool.settleIntent(intentId, solver2, proof, 0);
    }

    // ========== REFUND TESTS ==========

    function test_Refund() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        vm.warp(block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + 1);

        vm.expectEmit(true, false, false, false);
        emit IntentRefunded(intentId, TEST_AMOUNT);

        pool.refund(intentId);

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertTrue(intent.refunded);
        assertEq(token.balanceOf(user), TEST_AMOUNT);
    }

    function test_RevertWhen_Refund_NotExpired() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        vm.expectRevert(PrivateIntentPool.IntentNotExpired.selector);
        pool.refund(intentId);
    }

    // ========== MERKLE PROOF TESTS ==========

    function test_GenerateProof_SingleLeaf() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        (bytes32[] memory proof, uint256 index) = pool.generateCommitmentProof(commitment);
        
        assertEq(index, 0);
        assertEq(proof.length, 0); // Single leaf needs no proof
        assertEq(pool.getMerkleRoot(), commitment);
    }

    function test_GenerateProof_MultipleLeaves() public {
        // Create 3 intents
        token.mint(user, TEST_AMOUNT * 3);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT * 3);

        for (uint256 i = 0; i < 3; i++) {
            bytes32 s = keccak256(abi.encodePacked("secret", i));
            bytes32 n = keccak256(abi.encodePacked("nullifier", i));
            bytes32 id = keccak256(abi.encodePacked("intent", i));

            bytes32[4] memory inputs = [s, n, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
            bytes32 c = poseidon.poseidon(inputs);

            pool.createIntent(id, c, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        }
        vm.stopPrank();

        // Tree is padded to 4 leaves (power of 2)
        assertEq(pool.getCommitmentTreeSize(), 3);
        
        // Generate proof for first commitment
        bytes32 s0 = keccak256(abi.encodePacked("secret", uint256(0)));
        bytes32 n0 = keccak256(abi.encodePacked("nullifier", uint256(0)));
        bytes32[4] memory inputs0 = [s0, n0, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
        bytes32 c0 = poseidon.poseidon(inputs0);

        (bytes32[] memory proof, uint256 index) = pool.generateCommitmentProof(c0);
        
        assertEq(index, 0);
        assertEq(proof.length, 2); // Height is 2 (4 leaves = 2^2)
    }

    // ========== GAS BENCHMARKS ==========

    function test_Gas_CreateIntent() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(pool), TEST_AMOUNT);

        uint256 gasBefore = gasleft();
        pool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for createIntent (fixed Merkle):", gasUsed);
        vm.stopPrank();
    }
}