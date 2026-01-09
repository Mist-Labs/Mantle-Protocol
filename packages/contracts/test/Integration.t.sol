// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

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
 * @title IntegrationTest - FIXED for Power-of-2 Merkle Trees
 * @notice Tests use power-of-2 zero padding + OZ MerkleProof verification
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
        intentPool = new PrivateIntentPool(owner, relayer, feeCollector, address(poseidon));
        settlement = new PrivateSettlement(owner, relayer, feeCollector, address(poseidon));
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

    /**
     * @notice FIXED: Full flow test with proper proof generation
     */
    function test_FullFlow_CreateFillClaim() public {
        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);
        vm.stopPrank();

        bytes32 secret = keccak256("user_secret");
        bytes32 nullifier = keccak256("user_nullifier");
        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256(abi.encodePacked(block.timestamp, "intent1"));

        // 1. USER: Create intent
        vm.prank(user); 
        intentPool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);

        assertEq(token.balanceOf(address(intentPool)), TEST_AMOUNT);

        // 2. RELAYER: Generate proof using contract's method
        (bytes32[] memory sourceProof, uint256 leafIndex) = intentPool.generateCommitmentProof(commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        // 3. RELAYER: Sync and register on destination
        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, sourceProof, leafIndex);

        // 4. SOLVER: Fill intent
        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);

        assertEq(token.balanceOf(address(settlement)), TEST_AMOUNT);

        // 5. RELAYER: Sync fill tree root back to source
        bytes32[] memory destProof = settlement.generateFillProof(intentId);
        bytes32 destRoot = settlement.getMerkleRoot();
        
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        // 6. RELAYER: Settle intent
        vm.prank(relayer); 
        intentPool.settleIntent(intentId, solver, destProof, 0);

        uint256 poolFee = (TEST_AMOUNT * intentPool.FEE_BPS()) / 10000;
        assertEq(intentPool.getSolver(intentId), solver);

        // 7. USER: Claim withdrawal
        bytes32 authHash = keccak256(abi.encodePacked(intentId, nullifier, recipientAddr));
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(authHash);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(recipientPrivateKey, ethSignedHash);
        bytes memory claimAuth = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        settlement.claimWithdrawal(intentId, nullifier, recipientAddr, secret, claimAuth);

        uint256 settlementFee = (TEST_AMOUNT * settlement.FEE_BPS()) / 10000;
        uint256 expectedUserAmount = TEST_AMOUNT - settlementFee;
        assertEq(token.balanceOf(recipientAddr), expectedUserAmount);
        assertEq(token.balanceOf(feeCollector), poolFee + settlementFee);
    }

    /**
     * @notice FIXED: Multiple intents test with proper proofs
     */
    function test_FullFlow_MultipleIntents() public {
        token.mint(user, TEST_AMOUNT * 3);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT * 3);
        vm.stopPrank();

        for (uint256 i = 0; i < 3; i++) {
            bytes32 secret = keccak256(abi.encodePacked("secret", i));
            bytes32 nullifier = keccak256(abi.encodePacked("nullifier", i));
            bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
            bytes32 commitment = poseidon.poseidon(inputs);
            bytes32 intentId = keccak256(abi.encodePacked("intent", i));

            vm.prank(user); 
            intentPool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);

            (bytes32[] memory proof, uint256 leafIndex) = intentPool.generateCommitmentProof(commitment);
            bytes32 sourceRoot = intentPool.getMerkleRoot();

            vm.prank(relayer);
            settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

            vm.prank(relayer);
            settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, leafIndex);

            vm.prank(solver);
            settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
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
        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        intentPool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        bytes32 destRoot = intentId;
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        bytes32[] memory proof = new bytes32[](0);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(PrivateIntentPool.Unauthorized.selector);
        intentPool.settleIntent(intentId, solver, proof, 0);
    }

    function test_RevertWhen_NonRelayerRegistersIntent() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(SOURCE_CHAIN))];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        bytes32 sourceRoot = commitment;
        bytes32[] memory proof = new bytes32[](0);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(PrivateSettlement.Unauthorized.selector);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, proof, 0);
    }

    function test_Gas_CompleteFlow() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        bytes32[4] memory inputs = [secret, nullifier, bytes32(TEST_AMOUNT), bytes32(uint256(DEST_CHAIN))];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256("intent");

        token.mint(user, TEST_AMOUNT);
        vm.startPrank(user);
        token.approve(address(intentPool), TEST_AMOUNT);

        uint256 gasStart = gasleft();
        intentPool.createIntent(intentId, commitment, address(token), TEST_AMOUNT, address(token), TEST_AMOUNT - 1, DEST_CHAIN, user, 0);
        vm.stopPrank();

        uint256 gasAfterCreate = gasleft();
        console.log("Gas for createIntent:", gasStart - gasAfterCreate);

        (bytes32[] memory sourceProof, uint256 leafIndex) = intentPool.generateCommitmentProof(commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainRoot(SOURCE_CHAIN, sourceRoot);

        vm.prank(relayer);
        settlement.registerIntent(intentId, commitment, address(token), TEST_AMOUNT, SOURCE_CHAIN, uint64(block.timestamp + 1 hours), sourceRoot, sourceProof, leafIndex);

        gasStart = gasleft();
        vm.prank(solver);
        settlement.fillIntent(intentId, commitment, SOURCE_CHAIN, address(token), TEST_AMOUNT);
        uint256 gasAfterFill = gasleft();
        console.log("Gas for fillIntent:", gasStart - gasAfterFill);

        bytes32[] memory fillProof = settlement.generateFillProof(intentId);
        bytes32 destRoot = settlement.getMerkleRoot();
        vm.prank(relayer);
        intentPool.syncDestChainRoot(DEST_CHAIN, destRoot);

        gasStart = gasleft();
        vm.prank(relayer);
        intentPool.settleIntent(intentId, solver, fillProof, 0);
        console.log("Gas for settleIntent:", gasStart - gasleft());
    }
}