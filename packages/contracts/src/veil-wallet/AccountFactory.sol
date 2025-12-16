// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.20;

import "./SmartAccount.sol";
import "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";

/**
 * @title AccountFactory
 * @notice Factory contract for deploying counterfactual SmartAccount instances using CREATE2
 * @dev One account per owner, predictable addresses for gasless account creation
 */
contract AccountFactory {
    IEntryPoint public immutable entryPoint;

    event AccountCreated(address indexed account, address indexed owner, uint256 salt);

    /**
     * @notice Initialize the factory with the EntryPoint address
     * @param _entryPoint The canonical ERC-4337 EntryPoint address (0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789)
     */
    constructor(IEntryPoint _entryPoint) {
        entryPoint = _entryPoint;
    }

    /**
     * @notice Deploy a new SmartAccount if it doesn't exist
     * @param owner The owner address of the account
     * @param salt The salt for CREATE2 address determinism
     * @return ret The deployed SmartAccount instance
     */
    function createAccount(address owner, uint256 salt) external returns (SmartAccount ret) {
        address addr = getAddress(owner, salt);
        uint256 codeSize = addr.code.length;
        if (codeSize > 0) {
            return SmartAccount(payable(addr));
        }
        ret = new SmartAccount{salt: bytes32(salt)}(entryPoint, owner);
        emit AccountCreated(address(ret), owner, salt);
    }

    /**
     * @notice Get the counterfactual address for a given owner and salt
     * @param owner The owner address
     * @param salt The salt value
     * @return The predicted address of the SmartAccount
     */
    function getAddress(address owner, uint256 salt) public view returns (address) {
        bytes memory bytecode = abi.encodePacked(
            type(SmartAccount).creationCode,
            abi.encode(entryPoint, owner)
        );
        bytes32 hash = keccak256(
            abi.encodePacked(
                bytes1(0xff),
                address(this),
                salt,
                keccak256(bytecode)
            )
        );
        return address(uint160(uint256(hash)));
    }
}

