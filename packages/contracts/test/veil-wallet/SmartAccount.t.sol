// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {SmartAccount} from "../../src/veil-wallet/SmartAccount.sol";
import {IEntryPoint} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {PackedUserOperation} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

// This is a test mock that needs to be concrete, not abstract

contract MockERC20 is ERC20 {
    constructor() ERC20("Mock Token", "MOCK") {}
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/**
 * @title MockEntryPoint
 * @notice Simple mock EntryPoint for testing
 * @dev Concrete implementation of IEntryPoint interface for testing.
 *      This MUST be concrete (not abstract) as it's instantiated in tests.
 */
// slither-disable-next-line missing-inheritance
contract MockEntryPoint is IEntryPoint {
    mapping(address => mapping(uint192 => uint256)) public nonces;
    mapping(address => uint256) public balances;
    
    constructor() {
        // Concrete implementation - can be instantiated for testing
    }
    
    function getNonce(address sender, uint192 key) external view override returns (uint256) {
        return nonces[sender][key];
    }
    
    function incrementNonce(address sender, uint192 key) external {
        nonces[sender][key]++;
    }

    // IEntryPointStake functions
    function balanceOf(address account) external view override returns (uint256) {
        return balances[account];
    }

    function depositTo(address account) external payable override {
        balances[account] += msg.value;
    }

    function withdrawTo(address payable withdrawAddress, uint256 withdrawAmount) external override {
        require(balances[msg.sender] >= withdrawAmount, "Insufficient balance");
        balances[msg.sender] -= withdrawAmount;
        withdrawAddress.transfer(withdrawAmount);
    }

    function addStake(uint32) external payable override {
        balances[msg.sender] += msg.value;
    }

    function unlockStake() external override {
        // Stub implementation for testing
    }

    function withdrawStake(address payable withdrawAddress) external override {
        // For testing: withdraw all stake balance
        uint256 stakeBalance = balances[msg.sender];
        if (stakeBalance > 0) {
            balances[msg.sender] = 0;
            withdrawAddress.transfer(stakeBalance);
        }
    }

    // IEntryPoint functions
    function handleOps(PackedUserOperation[] calldata ops, address payable beneficiary) external override {
        for (uint256 i = 0; i < ops.length; i++) {
            nonces[ops[i].sender][0]++;
        }
        if (address(this).balance > 0 && beneficiary != address(0)) {
            beneficiary.transfer(address(this).balance);
        }
    }

    function handleAggregatedOps(
        IEntryPoint.UserOpsPerAggregator[] calldata opsPerAggregator,
        address payable beneficiary
    ) external override {
        for (uint256 i = 0; i < opsPerAggregator.length; i++) {
            for (uint256 j = 0; j < opsPerAggregator[i].userOps.length; j++) {
                nonces[opsPerAggregator[i].userOps[j].sender][0]++;
            }
        }
        if (address(this).balance > 0 && beneficiary != address(0)) {
            beneficiary.transfer(address(this).balance);
        }
    }
}

/**
 * @title SmartAccountTest
 * @notice Comprehensive test suite for SmartAccount contract
 */
contract SmartAccountTest is Test {
    SmartAccount public account;
    MockEntryPoint public entryPoint;
    MockERC20 public token;

    address public owner;
    uint256 public ownerPrivateKey;
    address public sessionKey;
    uint256 public sessionKeyPrivateKey;
    address public guardian;
    address public recipient;

    function setUp() public {
        // Setup accounts
        ownerPrivateKey = 0x1234;
        owner = vm.addr(ownerPrivateKey);
        
        sessionKeyPrivateKey = 0x5678;
        sessionKey = vm.addr(sessionKeyPrivateKey);
        
        guardian = makeAddr("guardian");
        recipient = makeAddr("recipient");

        // Deploy contracts
        entryPoint = new MockEntryPoint();
        account = new SmartAccount(IEntryPoint(address(entryPoint)), owner);
        token = new MockERC20();
        
        // Fund account
        vm.deal(address(account), 10 ether);
        token.mint(address(account), 1000 ether);
    }

    // ============ Constructor Tests ============

    function test_Constructor_SetsOwner() public {
        assertEq(account.owner(), owner);
    }

    function test_Constructor_SetsEntryPoint() public {
        assertEq(address(account.entryPoint()), address(entryPoint));
    }

    // ============ Session Key Tests ============

    function test_AddSessionKey_Success() public {
        uint256 validUntil = block.timestamp + 1 days;
        uint256 spendingLimit = 1 ether;

        vm.prank(owner);
        account.addSessionKey(sessionKey, validUntil, spendingLimit);

        (uint256 until, uint256 limit, uint256 spent) = account.sessionKeys(sessionKey);
        assertEq(until, validUntil);
        assertEq(limit, spendingLimit);
        assertEq(spent, 0);
    }

    function test_AddSessionKey_EmitsEvent() public {
        uint256 validUntil = block.timestamp + 1 days;
        uint256 spendingLimit = 1 ether;

        vm.expectEmit(true, false, false, false);
        emit SmartAccount.SessionKeyAdded(sessionKey, validUntil, spendingLimit);

        vm.prank(owner);
        account.addSessionKey(sessionKey, validUntil, spendingLimit);
    }

    function test_AddSessionKey_RevertsIfNotOwner() public {
        vm.expectRevert("Not owner");
        account.addSessionKey(sessionKey, block.timestamp + 1 days, 1 ether);
    }

    function test_AddSessionKey_RevertsIfZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert("Invalid session key");
        account.addSessionKey(address(0), block.timestamp + 1 days, 1 ether);
    }

    function test_AddSessionKey_RevertsIfExpiredTime() public {
        vm.prank(owner);
        vm.expectRevert("Invalid expiry time");
        account.addSessionKey(sessionKey, block.timestamp - 1, 1 ether);
    }

    function test_AddSessionKey_RevertsIfZeroLimit() public {
        vm.prank(owner);
        vm.expectRevert("Invalid spending limit");
        account.addSessionKey(sessionKey, block.timestamp + 1 days, 0);
    }

    function test_RevokeSessionKey_Success() public {
        // Add session key first
        vm.prank(owner);
        account.addSessionKey(sessionKey, block.timestamp + 1 days, 1 ether);

        // Revoke it
        vm.prank(owner);
        account.revokeSessionKey(sessionKey);

        (uint256 until, , ) = account.sessionKeys(sessionKey);
        assertEq(until, 0);
    }

    function test_RevokeSessionKey_EmitsEvent() public {
        // Add session key first
        vm.prank(owner);
        account.addSessionKey(sessionKey, block.timestamp + 1 days, 1 ether);

        vm.expectEmit(true, false, false, false);
        emit SmartAccount.SessionKeyRevoked(sessionKey);

        vm.prank(owner);
        account.revokeSessionKey(sessionKey);
    }

    function test_RevokeSessionKey_RevertsIfNotOwner() public {
        vm.expectRevert("Not owner");
        account.revokeSessionKey(sessionKey);
    }

    function test_RevokeSessionKey_RevertsIfNotExists() public {
        vm.prank(owner);
        vm.expectRevert("Session key not found");
        account.revokeSessionKey(sessionKey);
    }

    // ============ Guardian Tests ============

    function test_SetGuardian_Success() public {
        vm.prank(owner);
        account.setGuardian(guardian);

        assertEq(account.guardian(), guardian);
    }

    function test_SetGuardian_RevertsIfNotOwner() public {
        vm.expectRevert("Not owner");
        account.setGuardian(guardian);
    }

    function test_SetGuardian_RevertsIfZeroAddress() public {
        vm.prank(owner);
        vm.expectRevert("Invalid guardian");
        account.setGuardian(address(0));
    }

    function test_InitiateRecovery_Success() public {
        address newOwner = makeAddr("newOwner");
        
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        account.initiateRecovery(newOwner);

        (address recoveryOwner, uint256 timestamp, bool executed) = account.pendingRecovery();
        assertEq(recoveryOwner, newOwner);
        assertEq(timestamp, block.timestamp);
        assertFalse(executed);
    }

    function test_InitiateRecovery_EmitsEvent() public {
        address newOwner = makeAddr("newOwner");
        
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.expectEmit(true, false, false, false);
        emit SmartAccount.RecoveryInitiated(newOwner, block.timestamp);

        vm.prank(guardian);
        account.initiateRecovery(newOwner);
    }

    function test_InitiateRecovery_RevertsIfNotGuardian() public {
        vm.expectRevert("Only guardian");
        account.initiateRecovery(makeAddr("newOwner"));
    }

    function test_InitiateRecovery_RevertsIfZeroAddress() public {
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        vm.expectRevert("Invalid new owner");
        account.initiateRecovery(address(0));
    }

    function test_InitiateRecovery_RevertsIfSameOwner() public {
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        vm.expectRevert("Same owner");
        account.initiateRecovery(owner);
    }

    function test_ExecuteRecovery_Success() public {
        address newOwner = makeAddr("newOwner");
        
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        account.initiateRecovery(newOwner);

        vm.prank(guardian);
        account.executeRecovery();

        (, , bool executed) = account.pendingRecovery();
        assertTrue(executed);
    }

    function test_ExecuteRecovery_EmitsEvent() public {
        address newOwner = makeAddr("newOwner");
        
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        account.initiateRecovery(newOwner);

        vm.expectEmit(true, true, false, false);
        emit SmartAccount.RecoveryExecuted(owner, newOwner);

        vm.prank(guardian);
        account.executeRecovery();
    }

    function test_ExecuteRecovery_RevertsIfNotGuardian() public {
        vm.expectRevert("Only guardian");
        account.executeRecovery();
    }

    function test_ExecuteRecovery_RevertsIfNoPendingRecovery() public {
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        vm.expectRevert("No pending recovery");
        account.executeRecovery();
    }

    function test_ExecuteRecovery_RevertsIfAlreadyExecuted() public {
        address newOwner = makeAddr("newOwner");
        
        vm.prank(owner);
        account.setGuardian(guardian);

        vm.prank(guardian);
        account.initiateRecovery(newOwner);

        vm.prank(guardian);
        account.executeRecovery();

        vm.prank(guardian);
        vm.expectRevert("Recovery already executed");
        account.executeRecovery();
    }

    // ============ Execute Tests ============

    function test_Execute_Success() public {
        uint256 amount = 1 ether;
        bytes memory data = abi.encodeWithSignature("transfer(address,uint256)", recipient, amount);

        vm.prank(address(entryPoint));
        account.execute(address(token), 0, data);

        assertEq(token.balanceOf(recipient), amount);
    }

    function test_Execute_RevertsIfNotEntryPoint() public {
        vm.expectRevert();
        account.execute(address(token), 0, "");
    }

    function test_ExecuteBatch_Success() public {
        address[] memory targets = new address[](2);
        targets[0] = address(token);
        targets[1] = address(token);

        uint256[] memory values = new uint256[](2);
        values[0] = 0;
        values[1] = 0;

        bytes[] memory datas = new bytes[](2);
        datas[0] = abi.encodeWithSignature("transfer(address,uint256)", recipient, 100 ether);
        datas[1] = abi.encodeWithSignature("transfer(address,uint256)", makeAddr("recipient2"), 200 ether);

        vm.prank(address(entryPoint));
        account.executeBatch(targets, values, datas);

        assertEq(token.balanceOf(recipient), 100 ether);
        assertEq(token.balanceOf(makeAddr("recipient2")), 200 ether);
    }

    function test_ExecuteBatch_RevertsIfLengthMismatch() public {
        address[] memory targets = new address[](2);
        uint256[] memory values = new uint256[](1);
        bytes[] memory datas = new bytes[](2);

        vm.prank(address(entryPoint));
        vm.expectRevert("Array length mismatch");
        account.executeBatch(targets, values, datas);
    }

    // ============ Receive Tests ============

    function test_Receive_Ether() public {
        uint256 balanceBefore = address(account).balance;
        
        vm.deal(address(this), 1 ether);
        (bool success, ) = address(account).call{value: 1 ether}("");
        
        assertTrue(success);
        assertEq(address(account).balance, balanceBefore + 1 ether);
    }
}

