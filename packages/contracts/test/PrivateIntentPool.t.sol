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
 * @title PrivateIntentPoolTest
 * @notice Comprehensive test suite for PrivateIntentPool contract
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

    event IntentCreated(
        bytes32 indexed intentId,
        bytes32 indexed commitment,
        uint32 destChain,
        address token,
        uint256 amount
    );
    event IntentFilled(
        bytes32 indexed intentId,
        address indexed solver,
        address indexed token,
        uint256 amount
    );
    event IntentRefunded(bytes32 indexed intentId, uint256 amount);
    event RootSynced(uint32 indexed chainId, bytes32 root);
    event MerkleRootUpdated(bytes32 root);
    event TokenAdded(
        address indexed token,
        uint256 minAmount,
        uint256 maxAmount
    );
    event TokenRemoved(address indexed token);

    function setUp() public {
        poseidon = new PoseidonHasher();
        pool = new PrivateIntentPool(
            owner,
            relayer,
            feeCollector,
            address(poseidon)
        );
        token = new MockERC20();

        secret = keccak256("secret");
        nullifier = keccak256("nullifier");
        intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        commitment = poseidon.poseidon(inputs);

        token.mint(relayer, 1000 ether);
        vm.prank(relayer);
        token.approve(address(pool), type(uint256).max);

        // Add token to whitelist
        vm.prank(owner);
        pool.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    // ========== TOKEN WHITELIST TESTS ==========

    function test_AddSupportedToken() public {
        MockERC20 newToken = new MockERC20();

        vm.expectEmit(true, false, false, false);
        emit TokenAdded(address(newToken), 0.01 ether, 100 ether);

        vm.prank(owner);
        pool.addSupportedToken(
            address(newToken),
            0.01 ether, // minAmount
            100 ether, // maxAmount
            18 // decimals
        );

        assertTrue(pool.isTokenSupported(address(newToken)));

        address[] memory list = pool.getSupportedTokens();
        assertEq(list.length, 2);
        assertEq(list[1], address(newToken));

        assertEq(pool.getSupportedTokenCount(), 2);

        PrivateIntentPool.TokenConfig memory config = pool.getTokenConfig(
            address(newToken)
        );
        assertEq(config.minFillAmount, 0.01 ether);
        assertEq(config.maxFillAmount, 100 ether);
        assertEq(config.decimals, 18);
    }

    function test_RevertWhen_AddSupportedToken_NotOwner() public {
        MockERC20 newToken = new MockERC20();

        vm.prank(user);
        vm.expectRevert();
        pool.addSupportedToken(
            address(newToken),
            0.01 ether, // minAmount
            100 ether, // maxAmount
            18 // decimals
        );
    }

    function test_RevertWhen_AddSupportedToken_AlreadySupported() public {
        vm.prank(owner);
        vm.expectRevert(PrivateIntentPool.AlreadySupported.selector);
        pool.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    function test_RevertWhen_AddSupportedToken_ZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert(PrivateIntentPool.InvalidToken.selector);
        pool.addSupportedToken(address(0), 0.01 ether, 100 ether, 18);
    }

    function test_RemoveSupportedToken() public {
        vm.expectEmit(true, false, false, false);
        emit TokenRemoved(address(token));

        vm.prank(owner);
        pool.removeSupportedToken(address(token));

        assertFalse(pool.isTokenSupported(address(token)));

        address[] memory list = pool.getSupportedTokens();
        assertEq(list.length, 0);

        assertEq(pool.getSupportedTokenCount(), 0);
    }

    function test_RevertWhen_RemoveSupportedToken_NotOwner() public {
        vm.prank(user);
        vm.expectRevert();
        pool.removeSupportedToken(address(token));
    }

    function test_RevertWhen_RemoveSupportedToken_NotSupported() public {
        address notSupported = address(0xBEEF);

        vm.prank(owner);
        vm.expectRevert(PrivateIntentPool.TokenNotSupported.selector);
        pool.removeSupportedToken(notSupported);
    }

    function test_RemoveSupportedToken_MaintainsPackedArray() public {
        MockERC20 tokenB = new MockERC20();

        vm.startPrank(owner);
        pool.addSupportedToken(address(tokenB), 0.01 ether, 100 ether, 18);
        vm.stopPrank();

        vm.prank(owner);
        pool.removeSupportedToken(address(token));

        address[] memory list = pool.getSupportedTokens();

        assertEq(list.length, 1);
        assertEq(list[0], address(tokenB));
        assertTrue(pool.isTokenSupported(address(tokenB)));
        assertFalse(pool.isTokenSupported(address(token)));
    }

    function test_SupportedTokenQueries() public view {
        assertTrue(pool.isTokenSupported(address(token)));
        assertEq(pool.getSupportedTokenCount(), 1);

        address[] memory tokens = pool.getSupportedTokens();
        assertEq(tokens.length, 1);
        assertEq(tokens[0], address(token));
    }

    function test_TokenConfig_ValidatesBounds() public {
        PrivateIntentPool.TokenConfig memory config = pool.getTokenConfig(
            address(token)
        );

        assertEq(config.minFillAmount, 0.01 ether);
        assertEq(config.maxFillAmount, 100 ether);
        assertEq(config.decimals, 18);
        assertTrue(config.supported);
    }

    // 17. ADD: New test for updateTokenConfig
    function test_UpdateTokenConfig() public {
        vm.prank(owner);
        pool.updateTokenConfig(
            address(token),
            0.1 ether, // new minAmount
            50 ether // new maxAmount
        );

        PrivateIntentPool.TokenConfig memory config = pool.getTokenConfig(
            address(token)
        );
        assertEq(config.minFillAmount, 0.1 ether);
        assertEq(config.maxFillAmount, 50 ether);
    }

    // ========== INTENT CREATION TESTS ==========

    function test_CreateIntent() public {
        vm.startPrank(relayer);

        vm.expectEmit(true, true, false, true);
        emit IntentCreated(intentId, commitment, DEST_CHAIN, address(token), TEST_AMOUNT);

        vm.expectEmit(false, false, false, true);
        emit MerkleRootUpdated(commitment);

        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );

        vm.stopPrank();

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertEq(intent.commitment, commitment);
        assertEq(intent.token, address(token));
        assertEq(intent.amount, TEST_AMOUNT);
        assertEq(intent.destChain, DEST_CHAIN);
        assertEq(intent.refundTo, user);
        assertFalse(intent.filled);
        assertFalse(intent.refunded);

        assertTrue(pool.isCommitmentUsed(commitment));

        assertEq(token.balanceOf(address(pool)), TEST_AMOUNT);
    }

    function test_RevertWhen_CreateIntent_TokenNotSupported() public {
        MockERC20 unsupportedToken = new MockERC20();
        unsupportedToken.mint(relayer, 1000 ether);

        vm.prank(relayer);
        unsupportedToken.approve(address(pool), type(uint256).max);

        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.TokenNotSupported.selector);
        pool.createIntent(
            intentId,
            commitment,
            address(unsupportedToken),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );
    }

    function test_RevertWhen_CreateIntent_AmountTooSmall() public {
        vm.startPrank(relayer);

        uint256 smallAmount = 0.005 ether;

        vm.expectRevert(PrivateIntentPool.InvalidAmount.selector);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            smallAmount,
            DEST_CHAIN,
            user,
            0
        );

        vm.stopPrank();
    }

    function test_RevertWhen_CreateIntent_AmountTooLarge() public {
        vm.startPrank(relayer);

        uint256 largeAmount = 101 ether;

        vm.expectRevert(PrivateIntentPool.InvalidAmount.selector);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            largeAmount,
            DEST_CHAIN,
            user,
            0
        );

        vm.stopPrank();
    }

    function test_RevertWhen_CreateIntent_DuplicateCommitment() public {
        vm.startPrank(relayer);

        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );

        bytes32 intentId2 = keccak256("intent2");
        vm.expectRevert(PrivateIntentPool.DuplicateCommitment.selector);
        pool.createIntent(
            intentId2,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );

        vm.stopPrank();
    }

    function test_RevertWhen_CreateIntent_DuplicateIntentId() public {
        vm.startPrank(relayer);

        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );

        bytes32 secret2 = keccak256("secret2");
        bytes32 nullifier2 = keccak256("nullifier2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier2,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        vm.expectRevert(PrivateIntentPool.DuplicateCommitment.selector);
        pool.createIntent(
            intentId,
            commitment2,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );

        vm.stopPrank();
    }

    // ========== MARK FILLED TESTS ==========

    function test_MarkFilled() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.startPrank(solver);
        vm.expectEmit(true, true, false, false);
        emit IntentFilled(intentId, solver, address(token), TEST_AMOUNT);

        pool.markFilled(intentId, proof, 0);
        vm.stopPrank();

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertTrue(intent.filled);

        assertEq(pool.getSolver(intentId), solver);

        uint256 fee = (TEST_AMOUNT * pool.FEE_BPS()) / 10000;
        uint256 expectedSolverAmount = TEST_AMOUNT - fee;
        assertEq(token.balanceOf(solver), expectedSolverAmount);
        assertEq(token.balanceOf(feeCollector), fee);
    }

    function test_RevertWhen_MarkFilled_RootNotSynced() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(solver);
        vm.expectRevert(PrivateIntentPool.RootNotSynced.selector);
        pool.markFilled(intentId, proof, 0);
    }

    function test_RevertWhen_MarkFilled_IntentNotFound() public {
        bytes32 nonExistentId = keccak256("nonexistent");
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(solver);
        vm.expectRevert(PrivateIntentPool.IntentNotFound.selector);
        pool.markFilled(nonExistentId, proof, 0);
    }

    function test_RevertWhen_MarkFilled_AlreadyFilled() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(solver);
        pool.markFilled(intentId, proof, 0);

        address solver2 = makeAddr("solver2");
        vm.prank(solver2);
        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        pool.markFilled(intentId, proof, 0);
    }

    function test_RevertWhen_MarkFilled_InvalidProof() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = keccak256("destRoot");
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory invalidProof = new bytes32[](1);
        invalidProof[0] = keccak256("invalid");

        vm.prank(solver);
        vm.expectRevert(PrivateIntentPool.InvalidProof.selector);
        pool.markFilled(intentId, invalidProof, 0);
    }

    function test_MarkFilled_CompetingSolvers() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        address solver1 = makeAddr("solver1");
        vm.prank(solver1);
        pool.markFilled(intentId, proof, 0);

        assertEq(pool.getSolver(intentId), solver1);

        address solver2 = makeAddr("solver2");
        vm.prank(solver2);
        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        pool.markFilled(intentId, proof, 0);
    }

    // ========== REFUND TESTS ==========

    function test_Refund() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        vm.warp(block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + 1);

        vm.expectEmit(true, false, false, false);
        emit IntentRefunded(intentId, TEST_AMOUNT);

        pool.refund(intentId);

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertTrue(intent.refunded);

        assertEq(token.balanceOf(user), TEST_AMOUNT);
    }

    function test_RevertWhen_Refund_NotExpired() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        vm.expectRevert(PrivateIntentPool.IntentNotExpired.selector);
        pool.refund(intentId);
    }

    function test_RevertWhen_Refund_AlreadyFilled() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(solver);
        pool.markFilled(intentId, proof, 0);

        vm.warp(block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + 1);

        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        pool.refund(intentId);
    }

    function test_RevertWhen_Refund_AlreadyRefunded() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        vm.warp(block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + 1);
        pool.refund(intentId);

        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        pool.refund(intentId);
    }

    // ========== ROOT SYNC TESTS ==========

    function test_SyncDestChainRoot() public {
        bytes32 root = keccak256("root");

        vm.expectEmit(true, false, false, true);
        emit RootSynced(DEST_CHAIN, root);

        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, root);

        assertEq(pool.getDestChainRoot(DEST_CHAIN), root);
    }

    function test_RevertWhen_SyncDestChainRoot_Unauthorized() public {
        bytes32 root = keccak256("root");

        vm.prank(user);
        vm.expectRevert(PrivateIntentPool.Unauthorized.selector);
        pool.syncDestChainRoot(DEST_CHAIN, root);
    }

    function test_SyncDestChainRoot_UpdateExisting() public {
        bytes32 root1 = keccak256("root1");
        bytes32 root2 = keccak256("root2");

        vm.startPrank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, root1);
        assertEq(pool.getDestChainRoot(DEST_CHAIN), root1);

        pool.syncDestChainRoot(DEST_CHAIN, root2);
        assertEq(pool.getDestChainRoot(DEST_CHAIN), root2);
        vm.stopPrank();
    }

    // ========== MULTIPLE INTENTS TESTS ==========

    function test_CreateMultipleIntents() public {
        vm.startPrank(relayer);

        for (uint256 i = 0; i < 5; i++) {
            bytes32 s = keccak256(abi.encodePacked("secret", i));
            bytes32 n = keccak256(abi.encodePacked("nullifier", i));
            bytes32 id = keccak256(abi.encodePacked("intent", i));

            bytes32[4] memory inputs = [
                s,
                n,
                bytes32(TEST_AMOUNT),
                bytes32(uint256(DEST_CHAIN))
            ];
            bytes32 c = poseidon.poseidon(inputs);

            pool.createIntent(
                id,
                c,
                address(token),
                TEST_AMOUNT,
                DEST_CHAIN,
                user,
                0
            );

            assertTrue(pool.isCommitmentUsed(c));
        }

        vm.stopPrank();

        assertEq(token.balanceOf(address(pool)), TEST_AMOUNT * 5);
    }

    // ========== FUZZ TESTS ==========

    function testFuzz_CreateIntent_ValidAmount(uint256 amount) public {
        amount = bound(amount, 0.01 ether, 100 ether);

        bytes32 s = keccak256(abi.encodePacked("fuzz_secret"));
        bytes32 n = keccak256(abi.encodePacked("fuzz_nullifier"));
        bytes32 id = keccak256(abi.encodePacked("fuzz_intent"));

        bytes32[4] memory inputs = [
            s,
            n,
            bytes32(amount),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 c = poseidon.poseidon(inputs);

        token.mint(relayer, amount);

        vm.prank(relayer);
        pool.createIntent(id, c, address(token), amount, DEST_CHAIN, user, 0);

        PrivateIntentPool.Intent memory intent = pool.getIntent(id);
        assertEq(intent.amount, amount);
    }

    function testFuzz_Refund_AfterDeadline(uint256 timeAfterDeadline) public {
        timeAfterDeadline = bound(timeAfterDeadline, 1, 365 days);

        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        vm.warp(
            block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + timeAfterDeadline
        );

        pool.refund(intentId);

        PrivateIntentPool.Intent memory intent = pool.getIntent(intentId);
        assertTrue(intent.refunded);
    }

    // ========== GAS BENCHMARKS ==========

    function test_Gas_CreateIntent() public {
        vm.startPrank(relayer);

        uint256 gasBefore = gasleft();
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for createIntent:", gasUsed);
        vm.stopPrank();
    }

    function test_Gas_MarkFilled() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        pool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.startPrank(solver);
        uint256 gasBefore = gasleft();
        pool.markFilled(intentId, proof, 0);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for markFilled:", gasUsed);
        vm.stopPrank();
    }

    function test_Gas_Refund() public {
        vm.prank(relayer);
        pool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0 // customDeadline
        );

        vm.warp(block.timestamp + pool.DEFAULT_INTENT_TIMEOUT() + 1);

        uint256 gasBefore = gasleft();
        pool.refund(intentId);
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for refund:", gasUsed);
    }
}
