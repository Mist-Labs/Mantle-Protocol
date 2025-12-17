// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {AccountFactory} from "../../src/veil-wallet/AccountFactory.sol";
import {SmartAccount} from "../../src/veil-wallet/SmartAccount.sol";
import {IEntryPoint} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";

/**
 * @title AccountFactoryTest
 * @notice Comprehensive test suite for AccountFactory contract
 */
contract AccountFactoryTest is Test {
    AccountFactory public factory;
    IEntryPoint public entryPoint;

    address public owner1 = makeAddr("owner1");
    address public owner2 = makeAddr("owner2");

    // Mock EntryPoint for testing
    address public constant MOCK_ENTRYPOINT = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;

    function setUp() public {
        // Deploy mock EntryPoint or use address(0) for testing
        // In real tests, you'd deploy the actual EntryPoint
        entryPoint = IEntryPoint(MOCK_ENTRYPOINT);
        factory = new AccountFactory(entryPoint);
    }

    function test_Constructor_SetsEntryPoint() public {
        assertEq(address(factory.entryPoint()), address(entryPoint));
    }

    function test_GetAddress_PredictsCorrectAddress() public {
        uint256 salt = 12345;
        address predicted = factory.getAddress(owner1, salt);
        
        // Deploy and verify
        SmartAccount account = factory.createAccount(owner1, salt);
        assertEq(address(account), predicted);
    }

    function test_CreateAccount_DeploysNewAccount() public {
        uint256 salt = 1;
        address predicted = factory.getAddress(owner1, salt);
        
        // Account should not exist
        assertEq(predicted.code.length, 0);
        
        // Create account
        SmartAccount account = factory.createAccount(owner1, salt);
        
        // Verify account was deployed
        assertEq(address(account), predicted);
        assertGt(address(account).code.length, 0);
        assertEq(account.owner(), owner1);
    }

    function test_CreateAccount_EmitsEvent() public {
        uint256 salt = 2;
        address predicted = factory.getAddress(owner1, salt);
        
        vm.expectEmit(true, true, false, false);
        emit AccountFactory.AccountCreated(predicted, owner1, salt);
        
        factory.createAccount(owner1, salt);
    }

    function test_CreateAccount_ReturnsExistingAccount() public {
        uint256 salt = 3;
        
        // Create account first time
        SmartAccount account1 = factory.createAccount(owner1, salt);
        address accountAddr = address(account1);
        
        // Try to create again - should return existing
        SmartAccount account2 = factory.createAccount(owner1, salt);
        
        assertEq(address(account2), accountAddr);
        assertEq(address(account1), address(account2));
    }

    function test_CreateAccount_DifferentSalts_DifferentAddresses() public {
        uint256 salt1 = 100;
        uint256 salt2 = 200;
        
        address addr1 = factory.getAddress(owner1, salt1);
        address addr2 = factory.getAddress(owner1, salt2);
        
        assertNotEq(addr1, addr2);
        
        SmartAccount account1 = factory.createAccount(owner1, salt1);
        SmartAccount account2 = factory.createAccount(owner1, salt2);
        
        assertNotEq(address(account1), address(account2));
    }

    function test_CreateAccount_DifferentOwners_DifferentAddresses() public {
        uint256 salt = 500;
        
        address addr1 = factory.getAddress(owner1, salt);
        address addr2 = factory.getAddress(owner2, salt);
        
        assertNotEq(addr1, addr2);
        
        SmartAccount account1 = factory.createAccount(owner1, salt);
        SmartAccount account2 = factory.createAccount(owner2, salt);
        
        assertNotEq(address(account1), address(account2));
        assertEq(account1.owner(), owner1);
        assertEq(account2.owner(), owner2);
    }

    function test_GetAddress_Deterministic() public {
        uint256 salt = 999;
        address addr1 = factory.getAddress(owner1, salt);
        address addr2 = factory.getAddress(owner1, salt);
        
        // Should be deterministic
        assertEq(addr1, addr2);
    }

    function test_CreateAccount_MultipleAccounts() public {
        uint256[] memory salts = new uint256[](5);
        for (uint256 i = 0; i < 5; i++) {
            salts[i] = i + 1000;
        }
        
        address[] memory addresses = new address[](5);
        for (uint256 i = 0; i < 5; i++) {
            addresses[i] = factory.getAddress(owner1, salts[i]);
            factory.createAccount(owner1, salts[i]);
        }
        
        // Verify all addresses are different
        for (uint256 i = 0; i < 5; i++) {
            for (uint256 j = i + 1; j < 5; j++) {
                assertNotEq(addresses[i], addresses[j]);
            }
        }
    }

    function test_CreateAccount_ZeroSalt() public {
        uint256 salt = 0;
        address predicted = factory.getAddress(owner1, salt);
        SmartAccount account = factory.createAccount(owner1, salt);
        
        assertEq(address(account), predicted);
    }

    function test_CreateAccount_LargeSalt() public {
        uint256 salt = type(uint256).max;
        address predicted = factory.getAddress(owner1, salt);
        SmartAccount account = factory.createAccount(owner1, salt);
        
        assertEq(address(account), predicted);
    }
}

