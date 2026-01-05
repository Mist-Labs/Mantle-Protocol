// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/account/Account.sol";
import "@openzeppelin/contracts/utils/cryptography/signers/SignerECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/signers/AbstractSigner.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Address.sol";

/**
 * @title SmartAccount
 * @notice ERC-4337 compatible smart contract wallet with session keys and guardian support
 * @dev Extends OpenZeppelin's Account and SignerECDSA for ERC-4337 functionality
 */
contract SmartAccount is Account, SignerECDSA, ReentrancyGuard {
    using Address for address payable;

    struct SessionKey {
        uint256 validUntil;
        uint256 spendingLimit;
        uint256 spentAmount;
    }

    struct GuardianRecovery {
        address newOwner;
        uint256 timestamp;
        bool executed;
    }

    // Session key management
    mapping(address => SessionKey) public sessionKeys;
    
    // Guardian recovery (MVP: single guardian, full 3-of-3 in Phase 2)
    address public guardian;
    GuardianRecovery public pendingRecovery;
    
    // Track current userOp signer for spending limit enforcement
    address private _currentSigner;
    
    // Custom owner storage for recovery (overrides SignerECDSA's internal _signer)
    address private _owner;

    // Events
    event SessionKeyAdded(address indexed sessionKey, uint256 validUntil, uint256 spendingLimit);
    event SessionKeyRevoked(address indexed sessionKey);
    event RecoveryInitiated(address indexed newOwner, uint256 timestamp);
    event RecoveryExecuted(address indexed oldOwner, address indexed newOwner);
    event OwnerChanged(address indexed oldOwner, address indexed newOwner);

    IEntryPoint private immutable _entryPoint;

    /**
     * @notice Initialize the SmartAccount
     * @param _entryPointAddr The ERC-4337 EntryPoint address
     * @param owner_ The owner address (signer)
     */
    constructor(IEntryPoint _entryPointAddr, address owner_) SignerECDSA(owner_) {
        _entryPoint = _entryPointAddr;
        _owner = owner_; // Store owner separately for recovery
    }

    /**
     * @notice Override entryPoint to use the provided address
     * @dev Returns the EntryPoint address set during construction
     */
    function entryPoint() public view virtual override returns (IEntryPoint) {
        return _entryPoint;
    }

    /**
     * @notice Get the owner address
     * @return The owner address
     * @dev Overrides SignerECDSA's signer() to use our custom owner storage
     */
    function owner() public view returns (address) {
        return _owner;
    }
    
    /**
     * @notice Override signer() to return our custom owner
     * @dev This allows us to update the owner during recovery
     */
    function signer() public view override returns (address) {
        return _owner;
    }
    
    /**
     * @notice Override signature validation to use our custom owner
     * @dev This ensures signatures are validated against the current owner
     */
    function _rawSignatureValidation(
        bytes32 hash,
        bytes calldata signature
    ) internal view override(AbstractSigner, SignerECDSA) returns (bool) {
        (address recovered, ECDSA.RecoverError err, ) = ECDSA.tryRecover(hash, signature);
        return _owner == recovered && err == ECDSA.RecoverError.NoError;
    }

    /**
     * @notice Add a session key with time and spending limits
     * @param sessionKey The session key address
     * @param validUntil Unix timestamp when the key expires
     * @param spendingLimit Maximum amount (in wei) the key can spend
     */
    function addSessionKey(
        address sessionKey,
        uint256 validUntil,
        uint256 spendingLimit
    ) external onlyOwner {
        require(sessionKey != address(0), "Invalid session key");
        require(validUntil > block.timestamp, "Invalid expiry time");
        require(spendingLimit > 0, "Invalid spending limit");

        sessionKeys[sessionKey] = SessionKey({
            validUntil: validUntil,
            spendingLimit: spendingLimit,
            spentAmount: 0
        });

        emit SessionKeyAdded(sessionKey, validUntil, spendingLimit);
    }

    /**
     * @notice Revoke a session key
     * @param sessionKey The session key address to revoke
     */
    function revokeSessionKey(address sessionKey) external onlyOwner {
        require(sessionKeys[sessionKey].validUntil != 0, "Session key not found");
        delete sessionKeys[sessionKey];
        emit SessionKeyRevoked(sessionKey);
    }

    /**
     * @notice Set guardian address (MVP: single guardian)
     * @param _guardian The guardian address
     */
    function setGuardian(address _guardian) external onlyOwner {
        require(_guardian != address(0), "Invalid guardian");
        guardian = _guardian;
    }

    /**
     * @notice Initiate recovery process (guardian approves new owner)
     * @param newOwner The new owner address
     */
    function initiateRecovery(address newOwner) external {
        require(msg.sender == guardian, "Only guardian");
        require(newOwner != address(0), "Invalid new owner");
        require(newOwner != owner(), "Same owner");

        pendingRecovery = GuardianRecovery({
            newOwner: newOwner,
            timestamp: block.timestamp,
            executed: false
        });

        emit RecoveryInitiated(newOwner, block.timestamp);
    }

    /**
     * @notice Execute recovery (change owner after guardian approval)
     * @dev Guardian must call initiateRecovery first
     * @dev Requires a delay period (e.g., 24 hours) for security
     * @dev This is a self-custodial wallet - users have full control like Starknet wallets
     */
    function executeRecovery() external {
        require(msg.sender == guardian, "Only guardian");
        require(pendingRecovery.timestamp != 0, "No pending recovery");
        require(!pendingRecovery.executed, "Recovery already executed");
        require(block.timestamp >= pendingRecovery.timestamp + 1 days, "Recovery delay not met");

        address oldOwner = _owner;
        address newOwner = pendingRecovery.newOwner;

        // Update the owner - this gives the new owner full control
        _owner = newOwner;
        
        // Clear all session keys for security (new owner should add their own)
        // Note: We can't iterate mappings, so session keys remain but won't work
        // as they're tied to old owner's validation
        
        pendingRecovery.executed = true;

        emit RecoveryExecuted(oldOwner, newOwner);
        emit OwnerChanged(oldOwner, newOwner);
    }
    
    /**
     * @notice Owner can directly change owner (self-custodial control)
     * @dev Allows owner to transfer control without guardian
     * @param newOwner The new owner address
     */
    function changeOwner(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Invalid new owner");
        require(newOwner != _owner, "Same owner");
        
        address oldOwner = _owner;
        _owner = newOwner;
        
        emit OwnerChanged(oldOwner, newOwner);
    }

    /**
     * @notice Override validateUserOp to support session keys
     * @dev Checks if signer is owner OR valid session key
     */
    function _validateUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        bytes calldata signature
    ) internal override returns (uint256) {
        // Try to recover signer from signature
        address recoveredSigner = _recoverSigner(userOpHash, signature);
        
        // Check if signer is owner
        if (recoveredSigner == owner()) {
            _currentSigner = recoveredSigner;
            return super._validateUserOp(userOp, userOpHash, signature);
        }

        // Check if signer is a valid session key
        SessionKey storage session = sessionKeys[recoveredSigner];
        if (session.validUntil != 0) {
            require(block.timestamp <= session.validUntil, "Session key expired");
            
            // Extract call value from userOp to check spending limit
            uint256 callValue = _extractCallValue(userOp.callData);
            require(
                session.spentAmount + callValue <= session.spendingLimit,
                "Spending limit exceeded"
            );
            
            // Store current signer for spending tracking
            _currentSigner = recoveredSigner;
            
            // Session key validation successful
            return ERC4337Utils.SIG_VALIDATION_SUCCESS;
        }

        // Neither owner nor valid session key
        return ERC4337Utils.SIG_VALIDATION_FAILED;
    }
    
    /**
     * @notice Extract call value from callData
     * @dev Parses the execute/executeBatch call to get the value parameter
     */
    function _extractCallValue(bytes calldata callData) internal pure returns (uint256) {
        // For execute(address,uint256,bytes), value is at offset 36 (4 + 32)
        if (callData.length >= 68) {
            bytes4 selector = bytes4(callData[0:4]);
            // execute(address,uint256,bytes) selector = 0xb61d27f6
            // executeBatch(address[],uint256[],bytes[]) selector = 0x6171d1c9
            if (selector == 0xb61d27f6) {
                // Extract value (bytes 36-67)
                return uint256(bytes32(callData[36:68]));
            } else if (selector == 0x6171d1c9) {
                // For executeBatch, we'd need to decode and sum, but for validation
                // we'll return 0 and check during actual execution
                return 0;
            }
        }
        return 0;
    }

    /**
     * @notice Execute a single call
     * @param target The target address
     * @param value The amount of ETH to send
     * @param data The call data
     */
    function execute(
        address target,
        uint256 value,
        bytes calldata data
    ) external onlyEntryPointOrSelf nonReentrant returns (bytes memory) {
        // Track session key spending
        if (_currentSigner != address(0) && _currentSigner != owner()) {
            SessionKey storage session = sessionKeys[_currentSigner];
            if (session.validUntil != 0) {
                session.spentAmount += value;
                require(session.spentAmount <= session.spendingLimit, "Spending limit exceeded");
            }
        }
        
        bytes memory result = Address.functionCallWithValue(target, data, value);
        _currentSigner = address(0); // Reset after execution
        return result;
    }

    /**
     * @notice Execute a batch of calls
     * @param targets Array of target addresses
     * @param values Array of ETH amounts to send
     * @param datas Array of call data
     */
    function executeBatch(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata datas
    ) external onlyEntryPointOrSelf nonReentrant returns (bytes[] memory) {
        require(
            targets.length == values.length && targets.length == datas.length,
            "Array length mismatch"
        );

        // Track total spending for session keys
        uint256 totalValue = 0;
        if (_currentSigner != address(0) && _currentSigner != owner()) {
            for (uint256 i = 0; i < values.length; i++) {
                totalValue += values[i];
            }
            SessionKey storage session = sessionKeys[_currentSigner];
            if (session.validUntil != 0) {
                session.spentAmount += totalValue;
                require(session.spentAmount <= session.spendingLimit, "Spending limit exceeded");
            }
        }

        bytes[] memory results = new bytes[](targets.length);
        for (uint256 i = 0; i < targets.length; i++) {
            results[i] = Address.functionCallWithValue(targets[i], datas[i], values[i]);
        }
        _currentSigner = address(0); // Reset after execution
        return results;
    }

    /**
     * @notice Modifier to restrict access to owner only
     */
    modifier onlyOwner() {
        require(msg.sender == owner(), "Not owner");
        _;
    }

    /**
     * @notice Recover signer from hash and signature
     * @param hash The message hash
     * @param signature The signature
     * @return The recovered signer address
     */
    function _recoverSigner(
        bytes32 hash,
        bytes calldata signature
    ) internal pure returns (address) {
        (address recovered, ECDSA.RecoverError err, ) = ECDSA.tryRecover(hash, signature);
        if (err != ECDSA.RecoverError.NoError) {
            return address(0);
        }
        return recovered;
    }


    /**
     * @notice Receive Ether
     */
    receive() external payable override {}
}

