// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./Verifier.sol";

/**
 * @title VeilToken
 * @notice ERC20 token with privacy-preserving transfers using commitments
 * @dev MVP: Simple commitment scheme with Poseidon hashing and nullifiers
 * @dev On-chain commitments, off-chain encrypted data (wallet decrypts)
 */
contract VeilToken is ERC20, ReentrancyGuard {
    Verifier public immutable verifier;

    // Commitment tracking
    mapping(bytes32 => bool) public commitments;
    mapping(bytes32 => bool) public nullifiers;
    
    // Track commitment amounts (for MVP simplicity)
    mapping(bytes32 => uint256) public commitmentAmounts;

    // Events
    event PrivateTransfer(
        bytes32 indexed commitment,
        bytes32 indexed nullifier,
        address indexed recipient
    );
    
    event CommitmentClaimed(
        bytes32 indexed commitment,
        address indexed recipient,
        uint256 amount
    );

    event EncryptedBalanceUpdated(address indexed account, bytes encryptedData);

    /**
     * @notice Initialize the VeilToken
     * @param name Token name
     * @param symbol Token symbol
     * @param _verifier Address of the Verifier contract
     */
    constructor(
        string memory name,
        string memory symbol,
        address _verifier
    ) ERC20(name, symbol) {
        require(_verifier != address(0), "Invalid verifier");
        verifier = Verifier(_verifier);
    }

    /**
     * @notice Standard ERC20 transfer (transparent)
     * @param to Recipient address
     * @param amount Amount to transfer
     * @return success Whether the transfer succeeded
     */
    function transfer(
        address to,
        uint256 amount
    ) public virtual override returns (bool success) {
        return super.transfer(to, amount);
    }

    /**
     * @notice Private transfer using commitments
     * @dev For MVP: commitment = Poseidon(amount, blinding, recipient, nonce)
     * @param commitment The commitment hash
     * @param nullifier The nullifier hash (prevents double-spend)
     * @param amount The transfer amount (public for MVP simplicity)
     * @param proof The proof data (for MVP: simple hash verification)
     */
    function privateTransfer(
        bytes32 commitment,
        bytes32 nullifier,
        uint256 amount,
        bytes calldata proof
    ) external nonReentrant {
        // Prevent double-spending via nullifier
        require(!nullifiers[nullifier], "Nullifier already used");
        
        // Verify commitment hasn't been spent
        require(!commitments[commitment], "Commitment already spent");
        
        // For MVP: Simple proof verification
        // In production, this would verify a zkSNARK proof
        require(proof.length > 0, "Invalid proof");
        
        // Mark commitment as spent
        commitments[commitment] = true;
        commitmentAmounts[commitment] = amount;
        
        // Mark nullifier as used
        nullifiers[nullifier] = true;

        // For MVP: We don't actually transfer here
        // The actual transfer happens when recipient claims
        // This is a simplified model - in production, you'd have a more sophisticated system
        
        emit PrivateTransfer(commitment, nullifier, msg.sender);
    }

    /**
     * @notice Claim tokens from a commitment
     * @dev Recipient proves knowledge of the commitment secret to claim
     * @param commitment The commitment hash
     * @param amount The amount to claim
     * @param proof The proof that the caller knows the commitment secret
     */
    function claimFromCommitment(
        bytes32 commitment,
        uint256 amount,
        bytes calldata proof
    ) external nonReentrant {
        // Verify commitment exists and hasn't been claimed
        require(commitments[commitment], "Commitment not found");
        require(commitmentAmounts[commitment] == amount, "Amount mismatch");
        
        // For MVP: Simple proof check
        // In production, this would verify a Merkle proof or zkSNARK
        require(proof.length > 0, "Invalid proof");
        
        // Verify the commitment can be reconstructed (simplified for MVP)
        // In production, you'd verify a zkSNARK proof that the caller knows the secret
        
        // Mark commitment as spent (prevent double-claim)
        delete commitments[commitment];
        delete commitmentAmounts[commitment];
        
        // Transfer tokens to the claimer
        _mint(msg.sender, amount);
        
        emit CommitmentClaimed(commitment, msg.sender, amount);
    }

    /**
     * @notice Create a commitment for a private transfer
     * @dev This function helps users create commitments on-chain
     * @param inputs Array of 4 bytes32: [amount, blinding, recipient, nonce]
     * @return commitment The computed commitment hash
     */
    function createCommitment(
        bytes32[4] calldata inputs
    ) external view returns (bytes32 commitment) {
        return verifier.verifyCommitment(inputs);
    }

    /**
     * @notice Update encrypted balance (for wallet sync)
     * @dev This allows wallets to sync encrypted balance data off-chain
     * @param encryptedData The encrypted balance data
     */
    function updateEncryptedBalance(bytes calldata encryptedData) external {
        // For MVP: Just emit event, actual decryption happens off-chain
        // In production, this might store encrypted data or use a more sophisticated system
        emit EncryptedBalanceUpdated(msg.sender, encryptedData);
    }

    /**
     * @notice Check if a nullifier has been used
     * @param nullifier The nullifier hash to check
     * @return True if nullifier has been used
     */
    function isNullifierUsed(bytes32 nullifier) external view returns (bool) {
        return nullifiers[nullifier];
    }

    /**
     * @notice Check if a commitment exists and is unspent
     * @param commitment The commitment hash to check
     * @return True if commitment exists and is unspent
     */
    function isCommitmentValid(bytes32 commitment) external view returns (bool) {
        return commitments[commitment];
    }

    /**
     * @notice Mint tokens (for testing/initial supply)
     * @param to Address to mint to
     * @param amount Amount to mint
     */
    function mint(address to, uint256 amount) external {
        // For MVP: Allow anyone to mint (for testing)
        // In production, this would be restricted to authorized minters
        _mint(to, amount);
    }
}

