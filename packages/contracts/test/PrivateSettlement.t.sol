// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

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
    address public owner = makeAddr("owner");

    uint256 public leafIndex = 0;

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
        address indexed token,
        uint256 amount
    );
    event WithdrawalClaimed(
        bytes32 indexed intentId,
        bytes32 indexed nullifier,
        address indexed recipient,
        address token,
        uint256 amount
    );
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
        settlement = new PrivateSettlement(
            owner,
            relayer,
            feeCollector,
            address(poseidon)
        );
        token = new MockERC20();

        recipientAddr = vm.addr(recipientPrivateKey);

        secret = keccak256("secret");
        nullifier = keccak256("nullifier");
        intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        bytes32[4] memory inputs = [
            secret,
            nullifier,
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        commitment = poseidon.poseidon(inputs);

        token.mint(solver, 1000 ether);
        vm.prank(solver);
        token.approve(address(settlement), type(uint256).max);

        // Add token to whitelist
        vm.prank(owner);
        settlement.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
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
                next[i] = _hashPair(layer[2 * i], layer[2 * i + 1]);
            }
            if (n % 2 == 1) {
                next[nextN - 1] = layer[n - 1];
            }

            layer = next;
            n = nextN;
        }

        return layer[0];
    }

    function getMerkleProof(
        bytes32[] memory leaves,
        uint256 index
    ) internal pure returns (bytes32[] memory) {
        uint256 n = leaves.length;
        if (n == 0) return new bytes32[](0);
        if (n == 1) return new bytes32[](0);

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

    // ========== TOKEN WHITELIST TESTS ==========

    function test_AddSupportedToken() public {
        MockERC20 newToken = new MockERC20();

        vm.expectEmit(true, false, false, true);
        emit TokenAdded(address(newToken), 0.01 ether, 100 ether);

        vm.prank(owner);
        settlement.addSupportedToken(
            address(newToken),
            0.01 ether,
            100 ether,
            18
        );

        assertTrue(settlement.isTokenSupported(address(newToken)));

        address[] memory list = settlement.getSupportedTokens();
        assertEq(list.length, 2);
        assertEq(list[1], address(newToken));

        assertEq(settlement.getSupportedTokenCount(), 2);

        // Verify config
        PrivateSettlement.TokenConfig memory config = settlement.getTokenConfig(
            address(newToken)
        );
        assertEq(config.minFillAmount, 0.01 ether);
        assertEq(config.maxFillAmount, 100 ether);
        assertEq(config.decimals, 18);
    }

    function test_RevertWhen_AddSupportedToken_NotOwner() public {
        MockERC20 newToken = new MockERC20();

        vm.prank(solver);
        vm.expectRevert();
        settlement.addSupportedToken(
            address(newToken),
            0.01 ether,
            100 ether,
            18
        );
    }

    function test_RevertWhen_AddSupportedToken_AlreadySupported() public {
        vm.prank(owner);
        vm.expectRevert(PrivateSettlement.AlreadySupported.selector);
        settlement.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
    }

    function test_RevertWhen_AddSupportedToken_ZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert(PrivateSettlement.InvalidToken.selector);
        settlement.addSupportedToken(address(0), 0.01 ether, 100 ether, 18);
    }

    function test_RemoveSupportedToken() public {
        vm.expectEmit(true, false, false, false);
        emit TokenRemoved(address(token));

        vm.prank(owner);
        settlement.removeSupportedToken(address(token));

        assertFalse(settlement.isTokenSupported(address(token)));

        address[] memory list = settlement.getSupportedTokens();
        assertEq(list.length, 0);

        assertEq(settlement.getSupportedTokenCount(), 0);
    }

    function test_RevertWhen_RemoveSupportedToken_NotOwner() public {
        vm.prank(solver);
        vm.expectRevert();
        settlement.removeSupportedToken(address(token));
    }

    function test_RevertWhen_RemoveSupportedToken_NotSupported() public {
        address notSupported = address(0xBEEF);

        vm.prank(owner);
        vm.expectRevert(PrivateSettlement.TokenNotSupported.selector);
        settlement.removeSupportedToken(notSupported);
    }

    function test_RemoveSupportedToken_MaintainsPackedArray() public {
        MockERC20 tokenB = new MockERC20();

        vm.startPrank(owner);
        settlement.addSupportedToken(
            address(tokenB),
            0.01 ether,
            100 ether,
            18
        );
        vm.stopPrank();

        vm.prank(owner);
        settlement.removeSupportedToken(address(token));

        address[] memory list = settlement.getSupportedTokens();

        assertEq(list.length, 1);
        assertEq(list[0], address(tokenB));
        assertTrue(settlement.isTokenSupported(address(tokenB)));
        assertFalse(settlement.isTokenSupported(address(token)));
    }

    // ========== FILL INTENT TESTS ==========

    function test_FillIntent() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // Register intent first
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

    

        vm.expectEmit(true, true, false, true);
        emit IntentFilled(intentId, solver, address(token), TEST_AMOUNT);

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );



        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertEq(fill.solver, solver);
        assertEq(fill.token, address(token));
        assertEq(fill.amount, TEST_AMOUNT);
        assertEq(fill.sourceChain, SOURCE_CHAIN);
        assertFalse(fill.claimed);

        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);

        assertEq(settlement.getFillTreeSize(), 1);
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    function test_RevertWhen_FillIntent_TokenNotSupported() public {
        MockERC20 unsupportedToken = new MockERC20();
        unsupportedToken.mint(solver, 1000 ether);

        vm.prank(solver);
        unsupportedToken.approve(address(settlement), type(uint256).max);

        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

    function test_RevertWhen_FillIntent_AlreadyFilled() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.AlreadyFilled.selector);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );
    }

    function test_RevertWhen_FillIntent_InvalidProof() public {
        bytes32 sourceRoot = keccak256("some_other_root");
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory invalidProof = new bytes32[](1);
        invalidProof[0] = keccak256("invalid");

        vm.prank(relayer);
        vm.expectRevert(PrivateSettlement.InvalidProof.selector);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            invalidProof,
            0
        );
    }
    function test_FillIntent_MultipleSolvers() public {
        address solver2 = makeAddr("solver2");
        token.mint(solver2, 1000 ether);
        vm.prank(solver2);
        token.approve(address(settlement), type(uint256).max);

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

        bytes32[] memory proof0 = getMerkleProof(leaves, 0);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            proof0,
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

        bytes32 intentId2 = keccak256("intent2");

        bytes32[] memory proof1 = getMerkleProof(leaves, 1);

        vm.prank(relayer);
        settlement.registerIntent(
            intentId2,
            commitment2,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            proof1,
            1
        );

        vm.prank(solver2);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        assertEq(settlement.getFillTreeSize(), 2);
    }

    // ========== CLAIM WITHDRAWAL TESTS ==========

    function test_ClaimWithdrawal() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

        vm.expectEmit(true, true, true, true);
        emit WithdrawalClaimed(
            intentId,
            nullifier,
            recipientAddr,
            address(token),
            TEST_AMOUNT - ((TEST_AMOUNT * settlement.FEE_BPS()) / 10000)
        );

        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

        vm.stopPrank();

        PrivateSettlement.Fill memory fill = settlement.getFill(intentId);
        assertTrue(fill.claimed);
        assertTrue(settlement.isNullifierUsed(nullifier));

        uint256 fee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - fee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        assertEq(token.balanceOf(feeCollector), fee);
    }

    function test_RevertWhen_ClaimWithdrawal_Unauthorized() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

        vm.prank(solver); // THEN call fillIntent with ONLY these parameters:
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            ""
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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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

        vm.prank(solver); // THEN call fillIntent with ONLY these parameters:
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
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

        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

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
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
            0 // Fixed: leafIndex = 0 for single-leaf tree
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
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

        vm.prank(relayer);
        settlement.claimWithdrawal(
            intentId,
            nullifier,
            recipientAddr,
            secret,
            claimAuth
        );

        // Now try to reuse the same nullifier with a different intent
        bytes32 intentId2 = keccak256("intent2");
        bytes32 secret2 = keccak256("secret2");

        bytes32[4] memory inputs2 = [
            secret2,
            nullifier, // Same nullifier - this should fail!
            bytes32(TEST_AMOUNT),
            bytes32(uint256(SOURCE_CHAIN))
        ];
        bytes32 commitment2 = poseidon.poseidon(inputs2);

        bytes32[] memory leaves = new bytes32[](2);
        leaves[0] = commitment;
        leaves[1] = commitment2;
        bytes32 sourceRoot2 = buildMerkleRootFromLeaves(leaves);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot2);

        bytes32[] memory proof2 = getMerkleProof(leaves, 1);

        // Fixed: Add registerIntent before fillIntent
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
            1 // Fixed: leafIndex = 1 for second leaf in 2-leaf tree
        );

        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
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
            nullifier,
            recipientAddr,
            secret2,
            claimAuth2
        );
    }

    // test_RevertWhen_ClaimWithdrawal_InvalidCommitment
    function test_RevertWhen_ClaimWithdrawal_InvalidCommitment() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // ADD: Register intent first
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

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
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
            wrongSecret,
            claimAuth
        );
    }

    // test_RevertWhen_ClaimWithdrawal_InvalidSignature
    function test_RevertWhen_ClaimWithdrawal_InvalidSignature() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // ADD: Register intent first
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

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

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
        );
        bytes memory invalidClaimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
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

        vm.prank(solver);
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, root);
    }

    // ========== MERKLE TREE TESTS ==========

    // test_GenerateFillProof_SingleFill
    function test_GenerateFillProof_SingleFill() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // ADD: Register intent first
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

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        assertEq(fillProof.length, 0);
        assertEq(settlement.getMerkleRoot(), intentId);
    }

    // test_GenerateFillProof_MultipleFills
    function test_GenerateFillProof_MultipleFills() public {
        bytes32 sourceRoot1 = commitment;
        bytes32[] memory proof0 = new bytes32[](0);
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot1);

        // ADD: Register first intent
        vm.prank(relayer);
        settlement.registerIntent(
            intentId,
            commitment,
            address(token),
            TEST_AMOUNT,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot1,
            proof0,
            0
        );

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

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
        settlement.syncSourceChainRoot(2, sourceRoot2);

        // ADD: Register second intent
        vm.prank(relayer);
        settlement.registerIntent(
            intentId2,
            commitment2,
            address(token),
            TEST_AMOUNT,
            2,
            uint64(block.timestamp + 1 hours),
            sourceRoot2,
            proof1,
            0
        );

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            2,
            address(token),
            TEST_AMOUNT
        );

        bytes32 expectedRoot = _hashPair(intentId, intentId2);
        assertEq(settlement.getMerkleRoot(), expectedRoot);

        bytes32[] memory fillProof1 = settlement.generateFillProof(intentId);
        assertEq(fillProof1.length, 1);
        assertEq(fillProof1[0], intentId2);

        bytes32[] memory fillProof2 = settlement.generateFillProof(intentId2);
        assertEq(fillProof2.length, 1);
        assertEq(fillProof2[0], intentId);
    }

    // test_MerkleRootUpdates - Fix all three fillIntent calls
    function test_MerkleRootUpdates() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32 root1 = settlement.getMerkleRoot();
        assertEq(root1, bytes32(0));

        // ADD: Register first intent
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

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );

        bytes32 root2 = settlement.getMerkleRoot();
        assertEq(root2, intentId);

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

        // ADD: Register second intent
        vm.prank(relayer);
        settlement.registerIntent(
            intentId2,
            commitment2,
            address(token),
            TEST_AMOUNT,
            2,
            uint64(block.timestamp + 1 hours),
            sourceRoot2,
            proof,
            0
        );

        // FIXED: Correct fillIntent signature
        vm.prank(solver);
        settlement.fillIntent(
            intentId2,
            commitment2,
            2,
            address(token),
            TEST_AMOUNT
        );

        bytes32 expectedRoot3 = _hashPair(intentId, intentId2);
        bytes32 root3 = settlement.getMerkleRoot();
        assertEq(root3, expectedRoot3);
    }

    // ========== INTEGRATION TESTS ==========

    // test_FullFlowWithMultipleFills - Fix the fillIntent calls
    function test_FullFlowWithMultipleFills() public {
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

            // ADD: Register intent first
            vm.prank(relayer);
            settlement.registerIntent(
                ids[i],
                leaves[i],
                address(token),
                TEST_AMOUNT,
                SOURCE_CHAIN,
                uint64(block.timestamp + 1 hours),
                sourceRoot,
                p,
                i
            );

            vm.prank(solver);
            settlement.fillIntent(
                ids[i],
                leaves[i],
                SOURCE_CHAIN,
                address(token),
                TEST_AMOUNT
            );
        }

        assertEq(settlement.getFillTreeSize(), COUNT);
        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT * COUNT);

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

        assertTrue(settlement.getFill(targetIntent).claimed);
        assertTrue(settlement.isNullifierUsed(targetNullifier));
    }
    // ========== FUZZ TESTS ==========

    // testFuzz_FillIntent_ValidAmount
    function testFuzz_FillIntent_ValidAmount(uint256 amount) public {
        amount = bound(amount, 0.01 ether, 100 ether);

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

        bytes32 sourceRoot = c;
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        bytes32[] memory proof = new bytes32[](0);

        token.mint(solver, amount);

   
        vm.prank(relayer);
        settlement.registerIntent(
            id,
            c,
            address(token),
            amount,
            SOURCE_CHAIN,
            uint64(block.timestamp + 1 hours),
            sourceRoot,
            proof,
            0
        );

  
        vm.prank(solver);
        settlement.fillIntent(id, c, SOURCE_CHAIN, address(token), amount);

        PrivateSettlement.Fill memory fill = settlement.getFill(id);
        assertEq(fill.amount, amount);
        assertEq(token.balanceOf(address(settlement)), amount);
    }

    // ========== GAS BENCHMARKS ==========

    // test_Gas_FillIntent
    function test_Gas_FillIntent() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        // ADD: Register intent first
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
        uint256 gasBefore = gasleft();
        settlement.fillIntent(
            intentId,
            commitment,
            SOURCE_CHAIN,
            address(token),
            TEST_AMOUNT
        );
        uint256 gasUsed = gasBefore - gasleft();

        console.log("Gas used for fillIntent (1-leaf Merkle):", gasUsed);
    }

    // test_Gas_ClaimWithdrawal
    function test_Gas_ClaimWithdrawal() public {
        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

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
