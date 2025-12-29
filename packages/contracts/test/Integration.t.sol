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

    constructor(string memory name, string memory symbol, uint8 decimals_) ERC20(name, symbol) {
        _decimals = decimals_;
    }

    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

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

        // Add token to whitelists with proper parameters
        vm.startPrank(owner);
        intentPool.addSupportedToken(
            address(token),
            0.01 ether,  // minAmount
            100 ether,   // maxAmount
            18           // decimals
        );
        settlement.addSupportedToken(
            address(token),
            0.01 ether,  // minAmount
            100 ether,   // maxAmount
            18           // decimals
        );
        vm.stopPrank();
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
        bytes32 intentId = keccak256(
            abi.encodePacked(block.timestamp, "intent1")
        );

        // 2. RELAYER: Create intent on source chain (intentPool)
        vm.prank(relayer);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0  // customDeadline (0 = use default)
        );

        assertEq(token.balanceOf(address(intentPool)), TEST_AMOUNT);

        // 3. RELAYER: Sync commitment tree root to destination
        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // 4. RELAYER: Register intent on destination chain
        bytes32[] memory sourceProof = new bytes32[](0);

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
            0
        );

        // 5. SOLVER: Fill intent on destination chain (settlement)
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);

        // 6. RELAYER: Sync fill tree root back to source
        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        // 7. SOLVER: Claim reward by marking intent as filled on source chain
        bytes32[] memory destProof = settlement.generateFillProof(intentId);

        vm.prank(solver);
        intentPool.markFilled(intentId, solver, destProof, 0);

        // Verify solver received repayment on source chain
        uint256 poolFee = (TEST_AMOUNT * intentPool.FEE_BPS()) / 10000;
        assertEq(token.balanceOf(solver), 1000 ether - TEST_AMOUNT + (TEST_AMOUNT - poolFee));

        // Verify solver was recorded
        assertEq(intentPool.getSolver(intentId), solver);

        // 8. USER: Claim withdrawal on destination chain
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
                0
            );

            // Register and fill intent (single-leaf tree)
            bytes32 sourceRoot = commitment;
            vm.prank(relayer);
            settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

            bytes32[] memory proof = new bytes32[](0);

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
                0
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

        assertEq(
            intentPool.isCommitmentUsed(
                poseidon.poseidon(
                    [
                        keccak256(abi.encodePacked("secret", uint256(0))),
                        keccak256(abi.encodePacked("nullifier", uint256(0))),
                        bytes32(TEST_AMOUNT),
                        bytes32(uint256(DEST_CHAIN))
                    ]
                )
            ),
            true
        );

        assertEq(settlement.getFillTreeSize(), 3);
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT * 3);
    }

    // ========== TOKEN WHITELIST TESTS ===========

    function test_RevertWhen_CreateIntent_TokenNotSupported() public {
        MockERC20 unsupportedToken = new MockERC20("Unsupported", "UNSUP", 18);
        unsupportedToken.mint(relayer, 1000 ether);
        
        vm.prank(relayer);
        unsupportedToken.approve(address(intentPool), type(uint256).max);

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
        vm.expectRevert(PrivateIntentPool.TokenNotSupported.selector);
        intentPool.createIntent(
            intentId,
            commitment,
            address(unsupportedToken),
            TEST_AMOUNT,
            DEST_CHAIN,
            user,
            0
        );
    }

    function test_RevertWhen_FillIntent_TokenNotSupported() public {
        MockERC20 unsupportedToken = new MockERC20("Unsupported", "UNSUP", 18);
        unsupportedToken.mint(solver, 1000 ether);
        
        vm.prank(solver);
        unsupportedToken.approve(address(settlement), type(uint256).max);

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
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.TokenNotSupported.selector);
        settlement.registerIntent(
            intentId,
            commitment,
            address(unsupportedToken),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            proof,
            0
        );
    }

    function test_AddRemoveToken_WorksCorrectly() public {
        MockERC20 newToken = new MockERC20("New Token", "NEW", 6);

        // Add token with proper parameters
        vm.prank(owner);
        intentPool.addSupportedToken(
            address(newToken),
            1e6,      // 1 USDC minimum (6 decimals)
            1000e6,   // 1000 USDC maximum
            6         // decimals
        );
        
        assertTrue(intentPool.isTokenSupported(address(newToken)));
        assertEq(intentPool.getSupportedTokenCount(), 2); // token + newToken

        // Remove token
        vm.prank(owner);
        intentPool.removeSupportedToken(address(newToken));
        
        assertFalse(intentPool.isTokenSupported(address(newToken)));
        assertEq(intentPool.getSupportedTokenCount(), 1);
    }

    function test_TokenConfig_UpdateLimits() public {
        // Update existing token config
        vm.prank(owner);
        intentPool.updateTokenConfig(
            address(token),
            0.1 ether,  // new minAmount
            50 ether    // new maxAmount
        );

        PrivateIntentPool.TokenConfig memory config = intentPool.getTokenConfig(address(token));
        assertEq(config.minFillAmount, 0.1 ether);
        assertEq(config.maxFillAmount, 50 ether);
    }

    function test_RevertWhen_AmountBelowMinimum() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        uint256 tooSmallAmount = 0.005 ether; // Below 0.01 ether minimum
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(tooSmallAmount),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.InvalidAmount.selector);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            tooSmallAmount,
            DEST_CHAIN,
            user,
            0
        );
    }

    function test_RevertWhen_AmountAboveMaximum() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        uint256 tooLargeAmount = 150 ether; // Above 100 ether maximum
        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(tooLargeAmount),
            bytes32(uint256(DEST_CHAIN))
        ];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        token.mint(relayer, 150 ether);

        vm.prank(relayer);
        vm.expectRevert(PrivateIntentPool.InvalidAmount.selector);
        intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            tooLargeAmount,
            DEST_CHAIN,
            user,
            0
        );
    }

    // ========== PRIVACY TESTS ==========

    function test_Privacy_CommitmentHidesDetails() public view {
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
            0
        );

        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory proof = new bytes32[](0);

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
            0
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        // Claim once
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

        assertTrue(settlement.isNullifierUsed(nullifier));

        // Try to reuse nullifier
        bytes32 intentId2 = keccak256("intent2");
        bytes32 secret2 = keccak256("secret2");
        bytes32[4] memory inputs2 = [
            secret2,
            nullifier,  // Same nullifier
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
            0
        );

        bytes32 sourceRoot2 = commitment2;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot2);

        bytes32[] memory proof2 = new bytes32[](0);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId2,
            commitment2,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot2,
            proof2,
            0
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
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
            0
        );

        uint256 userBalanceBefore = token.balanceOf(user);

        vm.warp(block.timestamp + intentPool.DEFAULT_INTENT_TIMEOUT() + 1);

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
            0
        );

        vm.warp(block.timestamp + intentPool.DEFAULT_INTENT_TIMEOUT() + 1);
        intentPool.refund(intentId);

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        vm.prank(solver);
        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        intentPool.markFilled(intentId, solver, proof, 0);
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
                0
            );
        }

        assertEq(
            token.balanceOf(address(intentPool)),
            TEST_AMOUNT * intentCount
        );
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
            0
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
            0
        );

        uint256 poolBalance = token.balanceOf(address(intentPool));

        address attacker = makeAddr("attacker");

        vm.startPrank(attacker);

        bytes32[] memory proof = new bytes32[](0);

        vm.expectRevert(PrivateIntentPool.RootNotSynced.selector);
        intentPool.markFilled(intentId, solver, proof, 0);

        vm.stopPrank();

        assertEq(token.balanceOf(address(intentPool)), poolBalance);
    }

    function test_Security_MultipleSolversCompete() public {
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
            0
        );

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        address solver1 = makeAddr("solver1");
        vm.prank(solver1);
        intentPool.markFilled(intentId, solver1, proof, 0);

        address solver2 = makeAddr("solver2");
        vm.prank(solver2);
        vm.expectRevert(PrivateIntentPool.IntentAlreadyFilled.selector);
        intentPool.markFilled(intentId, solver2, proof, 0);

        assertEq(intentPool.getSolver(intentId), solver1);
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
            0
        );

        uint256 gasAfterCreate = gasleft();
        console.log("Gas for createIntent:", gasStart - gasAfterCreate);

        bytes32 sourceRoot = commitment;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory proof = new bytes32[](0);

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
            0
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

        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);

        gasStart = gasleft();
        vm.prank(solver);
        intentPool.markFilled(intentId, solver, fillProof, 0);
        console.log("Gas for markFilled:", gasStart - gasleft());
    }
}