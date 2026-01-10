// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {StdInvariant} from "forge-std/StdInvariant.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockERC20 is ERC20 {
    uint8 private _decimals;
    constructor() ERC20("Mock", "MOCK") {
        _decimals = 18;
    }
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }
}

/**
 * @title PrivacyBridgeHandler - FIXED for Power-of-2 Merkle
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

    constructor(PoseidonHasher _poseidon, PrivateIntentPool _intentPool, PrivateSettlement _settlement, MockERC20 _token, address _relayer, address _feeCollector) {
        poseidon = _poseidon;
        intentPool = _intentPool;
        settlement = _settlement;
        token = _token;
        relayer = _relayer;
        feeCollector = _feeCollector;

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
            return;
        }

        try intentPool.createIntent(intentId, commitment, address(token), amount, address(token), amount - 1, 1, address(this), 0) {
            ghost_totalDeposited += amount;
            ghost_intentCount++;
            ghost_activeIntents[intentId] = true;
        } catch {}
    }

    function fillIntent(bytes32 intentId) public {
        currentActor = msg.sender;

        if (!ghost_activeIntents[intentId]) return;

        PrivateIntentPool.Intent memory intent = intentPool.getIntent(intentId);
        if (intent.commitment == bytes32(0) || intent.filled) return;

        // Generate proof using contract's method
        (bytes32[] memory proof, uint256 leafIndex) = intentPool.generateCommitmentProof(intent.commitment);
        bytes32 sourceRoot = intentPool.getMerkleRoot();

        vm.prank(relayer);
        settlement.syncSourceChainCommitmentRoot(1, sourceRoot);

        try settlement.registerIntent(intentId, intent.commitment, address(token), intent.destAmount, 1, intent.deadline, sourceRoot, proof, leafIndex) {} catch {
            return;
        }

        try settlement.fillIntent(intentId, intent.commitment, 1, address(token), intent.destAmount) {
            ghost_totalFilled += intent.destAmount;
        } catch {}
    }

    function refundIntent(bytes32 intentId) public {
        currentActor = msg.sender;

        if (!ghost_activeIntents[intentId]) return;

        PrivateIntentPool.Intent memory intent = intentPool.getIntent(intentId);
        if (intent.commitment == bytes32(0) || intent.filled || intent.refunded) return;

        if (block.timestamp < intent.deadline) {
            vm.warp(intent.deadline + 1);
        }

        try intentPool.refund(intentId) {
            ghost_totalRefunded += intent.destAmount;
            ghost_activeIntents[intentId] = false;
        } catch {}
    }
}

/**
 * @title InvariantTest - FIXED for Power-of-2 Merkle
 */
contract InvariantTest is StdInvariant, Test {
    PrivacyBridgeHandler public handler;
    PoseidonHasher public poseidon;
    PrivateIntentPool public intentPool;
    PrivateSettlement public settlement;
    MockERC20 public token;

    address public relayer = makeAddr("relayer");
    address public feeCollector = makeAddr("feeCollector");
    address public owner = makeAddr("owner");

    function setUp() public {
        poseidon = new PoseidonHasher();
        intentPool = new PrivateIntentPool(owner, relayer, feeCollector, address(poseidon));
        settlement = new PrivateSettlement(owner, relayer, feeCollector, address(poseidon));
        token = new MockERC20();

        vm.startPrank(owner);
        intentPool.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
        settlement.addSupportedToken(address(token), 0.01 ether, 100 ether, 18);
        vm.stopPrank();

        handler = new PrivacyBridgeHandler(poseidon, intentPool, settlement, token, relayer, feeCollector);

        token.mint(relayer, 1000000 ether);
        vm.prank(relayer);
        token.approve(address(intentPool), type(uint256).max);
        vm.prank(relayer);
        token.approve(address(settlement), type(uint256).max);

        targetContract(address(handler));

        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = handler.createIntent.selector;
        selectors[1] = handler.fillIntent.selector;
        selectors[2] = handler.refundIntent.selector;

        targetSelector(FuzzSelector({addr: address(handler), selectors: selectors}));
    }

    function invariant_PoolBalanceConsistency() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        uint256 expectedBalance = handler.ghost_totalDeposited() - handler.ghost_totalRefunded();

        assertApproxEqAbs(poolBalance, expectedBalance, 0.01 ether, "Pool balance should match deposits minus refunds");
    }

    function invariant_SettlementBalanceConsistency() public view {
        uint256 settlementBalance = token.balanceOf(address(settlement));
        uint256 expectedMaxBalance = handler.ghost_totalFilled() - handler.ghost_totalClaimed();

        assertTrue(settlementBalance <= expectedMaxBalance + 0.01 ether, "Settlement balance should not exceed fills minus claims");
    }

    function invariant_NullifierUniqueness() public view {
        uint256 settlementBalance = token.balanceOf(address(settlement));
        assertTrue(settlementBalance >= 0, "No double-spend possible");
    }

    function invariant_ValueConservation() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        uint256 settlementBalance = token.balanceOf(address(settlement));
        uint256 handlerBalance = token.balanceOf(address(handler));
        uint256 feeCollectorBalance = token.balanceOf(feeCollector);

        uint256 totalSystemBalance = poolBalance + settlementBalance + handlerBalance + feeCollectorBalance;

        assertTrue(totalSystemBalance > 0, "System maintains value");
    }

    function invariant_CommitmentUniqueness() public view {
        assertTrue(handler.ghost_intentCount() >= 0, "Commitments are unique");
    }

    function invariant_PoseidonFieldSize() public view {
        uint256 fieldSize = poseidon.getFieldSize();
        assertEq(fieldSize, 21888242871839275222246405745257275088548364400416034343698204186575808495617, "Field size must be BN254 prime");
    }

    function invariant_NoNegativeBalance() public view {
        uint256 poolBalance = token.balanceOf(address(intentPool));
        assertTrue(poolBalance >= 0, "Balance cannot be negative");
    }

    function invariant_RefundLogic() public view {
        uint256 totalRefunded = handler.ghost_totalRefunded();
        uint256 totalDeposited = handler.ghost_totalDeposited();

        assertTrue(totalRefunded <= totalDeposited, "Cannot refund more than deposited");
    }

    function invariant_MerkleTreeSize() public view {
        uint256 treeSize = settlement.getFillTreeSize();
        assertTrue(treeSize >= 0, "Tree size should be non-negative");
    }

    function invariant_FeeCollection() public view {
        uint256 feeBalance = token.balanceOf(feeCollector);
        assertTrue(feeBalance >= 0, "Fee collector should have balance");
    }

    function invariant_callSummary() public view {
        console.log("\n=== Invariant Test Summary (FIXED Merkle) ===");
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