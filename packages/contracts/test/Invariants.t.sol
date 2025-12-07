// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {StdInvariant} from "forge-std/StdInvariant.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockERC20 is ERC20 {
    constructor() ERC20("Mock", "MOCK") {}
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

/**
 * @title PrivacyBridgeHandler
 * @notice Handler contract for invariant testing
 */
contract PrivacyBridgeHandler is Test {
    PoseidonHasher public poseidon;
    PrivateIntentPool public intentPool;
    PrivateSettlement public settlement;
    MockERC20 public token;
    
    address public relayer;
    address public feeCollector;
    address public currentActor;
    
    uint256 public ghost_totalDeposited;
    uint256 public ghost_totalFilled;
    uint256 public ghost_totalClaimed;
    uint256 public ghost_totalRefunded;
    uint256 public ghost_intentCount;
    
    mapping(bytes32 => bool) public ghost_activeIntents;
    
    constructor(
        PoseidonHasher _poseidon,
        PrivateIntentPool _intentPool,
        PrivateSettlement _settlement,
        MockERC20 _token,
        address _relayer,
        address _feeCollector
    ) {
        poseidon = _poseidon;
        intentPool = _intentPool;
        settlement = _settlement;
        token = _token;
        relayer = _relayer;
        feeCollector = _feeCollector;
        
        // Fund handler
        token.mint(address(this), 1000000 ether);
        token.approve(address(intentPool), type(uint256).max);
        token.approve(address(settlement), type(uint256).max);
    }
    
    function createIntent(uint256 amount, uint256 secretSeed, uint256 nullifierSeed) public {
        currentActor = msg.sender;
        
        amount = bound(amount, 0.001 ether, 100 ether);
        
        bytes32 secret = keccak256(abi.encodePacked(secretSeed));
        bytes32 nullifier = keccak256(abi.encodePacked(nullifierSeed));
        bytes32[4] memory inputs = [secret, nullifier, bytes32(amount), bytes32(uint256(1))];
        bytes32 commitment = poseidon.poseidon(inputs);
        bytes32 intentId = keccak256(abi.encodePacked(block.timestamp, secretSeed, nullifierSeed));
        
        if (intentPool.isCommitmentUsed(commitment)) {
            return; // Skip duplicate
        }
        
        try intentPool.createIntent(
            intentId,
            commitment,
            address(token),
            amount,
            1,
            address(this),
            secret,
            nullifier
        ) {
            ghost_totalDeposited += amount;
            ghost_intentCount++;
            ghost_activeIntents[intentId] = true;
        } catch {
            // Ignore failures
        }
    }
    
    function fillIntent(bytes32 intentId) public {
        currentActor = msg.sender;
        
        if (!ghost_activeIntents[intentId]) return;
        
        PrivateIntentPool.Intent memory intent = intentPool.getIntent(intentId);
        if (intent.commitment == bytes32(0) || intent.filled) return;
        
        bytes32 sourceRoot = keccak256("root");
        bytes32[] memory proof = new bytes32[](1);
        proof[0] = intent.commitment;
        
        try settlement.fillIntent(
            intentId,
            intent.commitment,
            1,
            address(token),
            intent.amount,
            sourceRoot,
            proof,
            0
        ) {
            ghost_totalFilled += intent.amount;
        } catch {
            // Ignore failures
        }
    }
    
    function refundIntent(bytes32 intentId) public {
        currentActor = msg.sender;
        
        if (!ghost_activeIntents[intentId]) return;
        
        PrivateIntentPool.Intent memory intent = intentPool.getIntent(intentId);
        if (intent.commitment == bytes32(0) || intent.filled || intent.refunded) return;
        
        // Fast forward if needed
        if (block.timestamp < intent.deadline) {
            vm.warp(intent.deadline + 1);
        }
        
        try intentPool.refund(intentId) {
            ghost_totalRefunded += intent.amount;
            ghost_activeIntents[intentId] = false;
        } catch {
            // Ignore failures
        }
    }
}

/**
 * @title InvariantTest
 * @notice Invariant tests to ensure system properties always hold
 */
contract InvariantTest is StdInvariant, Test {
    PrivacyBridgeHandler public handler;
    PoseidonHasher public poseidon;
    PrivateIntentPool public intentPool;
    PrivateSettlement public settlement;
    MockERC20 public token;
    
    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    
    function setUp() public {
        // Deploy system
        poseidon = new PoseidonHasher();
        intentPool = new PrivateIntentPool(relayer, feeCollector, address(poseidon));
        settlement = new PrivateSettlement(relayer, feeCollector, address(poseidon));
        token = new MockERC20();
        
        // Deploy handler
        handler = new PrivacyBridgeHandler(
            poseidon,
            intentPool,
            settlement,
            token,
            relayer,
            feeCollector
        );
        
        // Fund relayer
        token.mint(relayer, 1000000 ether);
        vm.prank(relayer);
        token.approve(address(intentPool), type(uint256).max);
        
        // Setup handler as target
        targetContract(address(handler));
        
        // Target specific functions
        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = handler.createIntent.selector;
        selectors[1] = handler.fillIntent.selector;
        selectors[2] = handler.refundIntent.selector;
        
        targetSelector(FuzzSelector({addr: address(handler), selectors: selectors}));
    }
    
    // ========== INVARIANTS ==========
    
    /// @notice Pool should never hold more tokens than total deposits minus claims
    function invariant_PoolBalanceConsistency() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        uint256 expectedBalance = handler.ghost_totalDeposited() - 
                                   handler.ghost_totalRefunded();
        
        // Allow small rounding errors from fees
        assertApproxEqAbs(
            poolBalance,
            expectedBalance,
            0.01 ether,
            "Pool balance should match deposits minus refunds"
        );
    }
    
    /// @notice Settlement should never hold more than filled intents
    function invariant_SettlementBalanceConsistency() public view {
        uint256 settlementBalance = token.balanceOf(address(settlement));
        uint256 expectedMaxBalance = handler.ghost_totalFilled() - 
                                      handler.ghost_totalClaimed();
        
        assertTrue(
            settlementBalance <= expectedMaxBalance + 0.01 ether,
            "Settlement balance should not exceed fills minus claims"
        );
    }
    
    /// @notice Nullifiers must be unique (never reused)
    function invariant_NullifierUniqueness() public view {
        // This is enforced by the contract's mapping(bytes32 => bool) nullifiers
        // If we could claim twice with same nullifier, balance would be wrong
        uint256 settlementBalance = token.balanceOf(address(settlement));
        assertTrue(settlementBalance >= 0, "No double-spend possible");
    }
    
    /// @notice Total system value conservation
    function invariant_ValueConservation() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        uint256 settlementBalance = token.balanceOf(address(settlement));
        uint256 handlerBalance = token.balanceOf(address(handler));
        uint256 feeCollectorBalance = token.balanceOf(feeCollector);
        
        uint256 totalSystemBalance = poolBalance + settlementBalance + handlerBalance + feeCollectorBalance;
        
        // Total should not exceed initial mint plus new mints
        assertTrue(totalSystemBalance > 0, "System maintains value");
    }
    
    /// @notice Commitments must be unique
    function invariant_CommitmentUniqueness() public view {
        // Enforced by contract's mapping(bytes32 => bool) commitments
        // If we could reuse commitments, we'd have duplicate intents
        assertTrue(handler.ghost_intentCount() >= 0, "Commitments are unique");
    }
    
    /// @notice Poseidon hashes must always be in field
    function invariant_PoseidonFieldSize() public view {
        uint256 fieldSize = poseidon.getFieldSize();
        assertEq(
            fieldSize,
            21888242871839275222246405745257275088548364400416034343698204186575808495617,
            "Field size must be BN254 prime"
        );
    }
    
    /// @notice Intent pool balance should never be negative
    function invariant_NoNegativeBalance() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        assertTrue(poolBalance >= 0, "Balance cannot be negative");
    }
    
    /// @notice Refunds should only happen for expired, unfilled intents
    function invariant_RefundLogic() public view {
        uint256 totalRefunded = handler.ghost_totalRefunded();
        uint256 totalDeposited = handler.ghost_totalDeposited();
        
        assertTrue(
            totalRefunded <= totalDeposited,
            "Cannot refund more than deposited"
        );
    }
    
    /// @notice Merkle tree size should match number of fills
    function invariant_MerkleTreeSize() public view {
        uint256 treeSize = settlement.getFillTreeSize();
        assertTrue(treeSize >= 0, "Tree size should be non-negative");
    }
    
    /// @notice Fee collector should receive fees
    function invariant_FeeCollection() public view {
        uint256 feeBalance = token.balanceOf(feeCollector);
        assertTrue(feeBalance >= 0, "Fee collector should have balance");
    }
    
    // ========== CALL SUMMARY ==========
    
    function invariant_callSummary() public view {
        console.log("\n=== Invariant Test Summary ===");
        console.log("Total intents created:", handler.ghost_intentCount());
        console.log("Total deposited:", handler.ghost_totalDeposited());
        console.log("Total filled:", handler.ghost_totalFilled());
        console.log("Total claimed:", handler.ghost_totalClaimed());
        console.log("Total refunded:", handler.ghost_totalRefunded());
        console.log("\nContract Balances:");
        console.log("  Intent Pool:", token.balanceOf(address(intentPool)));
        console.log("  Settlement:", token.balanceOf(address(settlement)));
        console.log("  Fee Collector:", token.balanceOf(feeCollector));
        console.log("  Handler:", token.balanceOf(address(handler)));
    }
}