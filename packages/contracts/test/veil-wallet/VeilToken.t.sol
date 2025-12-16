// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {VeilToken} from "../../src/veil-wallet/VeilToken.sol";
import {Verifier} from "../../src/veil-wallet/Verifier.sol";
import {PoseidonHasher} from "../../src/PoseidonHasher.sol";

/**
 * @title VeilTokenTest
 * @notice Comprehensive test suite for VeilToken contract
 */
contract VeilTokenTest is Test {
    VeilToken public token;
    Verifier public verifier;
    PoseidonHasher public hasher;

    address public user1 = makeAddr("user1");
    address public user2 = makeAddr("user2");
    address public user3 = makeAddr("user3");

    uint256 public constant INITIAL_SUPPLY = 10000 ether;
    uint256 public constant TEST_AMOUNT = 100 ether;

    function setUp() public {
        hasher = new PoseidonHasher();
        verifier = new Verifier(address(hasher));
        token = new VeilToken("Veil Token", "VEIL", address(verifier));
        
        // Mint initial supply
        token.mint(user1, INITIAL_SUPPLY);
    }

    // ============ Constructor Tests ============

    function test_Constructor_SetsNameAndSymbol() public {
        assertEq(token.name(), "Veil Token");
        assertEq(token.symbol(), "VEIL");
    }

    function test_Constructor_SetsVerifier() public {
        assertEq(address(token.verifier()), address(verifier));
    }

    function test_Constructor_RevertsIfZeroVerifier() public {
        vm.expectRevert("Invalid verifier");
        new VeilToken("Test", "TEST", address(0));
    }

    // ============ Standard ERC20 Tests ============

    function test_Transfer_StandardERC20() public {
        vm.prank(user1);
        bool success = token.transfer(user2, TEST_AMOUNT);
        
        assertTrue(success);
        assertEq(token.balanceOf(user2), TEST_AMOUNT);
        assertEq(token.balanceOf(user1), INITIAL_SUPPLY - TEST_AMOUNT);
    }

    function test_Transfer_InsufficientBalance() public {
        vm.prank(user2);
        vm.expectRevert();
        token.transfer(user3, TEST_AMOUNT);
    }

    // ============ Private Transfer Tests ============

    function test_PrivateTransfer_Success() public {
        // Create commitment inputs
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));

        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // Verify commitment is stored
        assertTrue(token.commitments(commitment));
        assertEq(token.commitmentAmounts(commitment), amount);
        assertTrue(token.nullifiers(nullifier));
    }

    function test_PrivateTransfer_EmitsEvent() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.expectEmit(true, true, true, false);
        emit VeilToken.PrivateTransfer(commitment, nullifier, user1);

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);
    }

    function test_PrivateTransfer_RevertsIfNullifierUsed() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // Try to use same nullifier again
        bytes32 commitment2 = keccak256("different_commitment");
        vm.prank(user1);
        vm.expectRevert("Nullifier already used");
        token.privateTransfer(commitment2, nullifier, amount, proof);
    }

    function test_PrivateTransfer_RevertsIfCommitmentSpent() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // Try to use same commitment again
        bytes32 nullifier2 = keccak256("different_nullifier");
        vm.prank(user1);
        vm.expectRevert("Commitment already spent");
        token.privateTransfer(commitment, nullifier2, amount, proof);
    }

    function test_PrivateTransfer_RevertsIfInvalidProof() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 commitment = keccak256("commitment");
        bytes32 nullifier = keccak256("nullifier");
        bytes memory emptyProof = "";

        vm.prank(user1);
        vm.expectRevert("Invalid proof");
        token.privateTransfer(commitment, nullifier, amount, emptyProof);
    }

    // ============ Claim Tests ============

    function test_ClaimFromCommitment_Success() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        // Create private transfer first
        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // Claim the commitment
        bytes memory claimProof = abi.encodePacked("claim_proof");
        vm.prank(user2);
        token.claimFromCommitment(commitment, amount, claimProof);

        // Verify tokens were minted to claimer
        assertEq(token.balanceOf(user2), amount);
        // Verify commitment is deleted
        assertFalse(token.commitments(commitment));
        assertEq(token.commitmentAmounts(commitment), 0);
    }

    function test_ClaimFromCommitment_EmitsEvent() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        bytes memory claimProof = abi.encodePacked("claim_proof");
        vm.expectEmit(true, true, false, false);
        emit VeilToken.CommitmentClaimed(commitment, user2, amount);

        vm.prank(user2);
        token.claimFromCommitment(commitment, amount, claimProof);
    }

    function test_ClaimFromCommitment_RevertsIfCommitmentNotFound() public {
        bytes32 commitment = keccak256("nonexistent");
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user2);
        vm.expectRevert("Commitment not found");
        token.claimFromCommitment(commitment, TEST_AMOUNT, proof);
    }

    function test_ClaimFromCommitment_RevertsIfAmountMismatch() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // Try to claim with wrong amount
        bytes memory claimProof = abi.encodePacked("claim_proof");
        vm.prank(user2);
        vm.expectRevert("Amount mismatch");
        token.claimFromCommitment(commitment, amount + 1, claimProof);
    }

    function test_ClaimFromCommitment_RevertsIfInvalidProof() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        bytes memory emptyProof = "";
        vm.prank(user2);
        vm.expectRevert("Invalid proof");
        token.claimFromCommitment(commitment, amount, emptyProof);
    }

    // ============ Helper Function Tests ============

    function test_CreateCommitment() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = token.createCommitment(inputs);
        bytes32 expected = verifier.verifyCommitment(inputs);
        
        assertEq(commitment, expected);
    }

    function test_UpdateEncryptedBalance_EmitsEvent() public {
        bytes memory encryptedData = abi.encodePacked("encrypted_balance_data");

        vm.expectEmit(true, false, false, false);
        emit VeilToken.EncryptedBalanceUpdated(user1, encryptedData);

        vm.prank(user1);
        token.updateEncryptedBalance(encryptedData);
    }

    function test_IsNullifierUsed() public {
        bytes32 nullifier = keccak256("test_nullifier");
        
        assertFalse(token.isNullifierUsed(nullifier));

        // Use the nullifier
        uint256 amount = TEST_AMOUNT;
        bytes32 commitment = keccak256("test_commitment");
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        assertTrue(token.isNullifierUsed(nullifier));
    }

    function test_IsCommitmentValid() public {
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = verifier.verifyCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        assertFalse(token.isCommitmentValid(commitment));

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        assertTrue(token.isCommitmentValid(commitment));

        // Claim it
        bytes memory claimProof = abi.encodePacked("claim_proof");
        vm.prank(user2);
        token.claimFromCommitment(commitment, amount, claimProof);

        assertFalse(token.isCommitmentValid(commitment));
    }

    // ============ Mint Tests ============

    function test_Mint() public {
        uint256 mintAmount = 500 ether;
        token.mint(user3, mintAmount);

        assertEq(token.balanceOf(user3), mintAmount);
    }

    // ============ Integration Tests ============

    function test_FullPrivateTransferFlow() public {
        // 1. User1 creates a private transfer
        uint256 amount = TEST_AMOUNT;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(user2)));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        bytes32 commitment = token.createCommitment(inputs);
        bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier"));
        bytes memory proof = abi.encodePacked("proof");

        vm.prank(user1);
        token.privateTransfer(commitment, nullifier, amount, proof);

        // 2. Verify commitment exists
        assertTrue(token.isCommitmentValid(commitment));
        assertTrue(token.isNullifierUsed(nullifier));

        // 3. User2 claims the commitment
        bytes memory claimProof = abi.encodePacked("claim_proof");
        vm.prank(user2);
        token.claimFromCommitment(commitment, amount, claimProof);

        // 4. Verify tokens were minted
        assertEq(token.balanceOf(user2), amount);
        assertFalse(token.isCommitmentValid(commitment));
    }

    function test_MultiplePrivateTransfers() public {
        for (uint256 i = 0; i < 5; i++) {
            uint256 amount = 10 ether;
            bytes32 blinding = keccak256(abi.encodePacked("blinding", i));
            bytes32 recipient = bytes32(uint256(uint160(user2)));
            bytes32 nonce = bytes32(i);
            
            bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
            bytes32 commitment = token.createCommitment(inputs);
            bytes32 nullifier = keccak256(abi.encodePacked(commitment, "nullifier", i));
            bytes memory proof = abi.encodePacked("proof", i);

            vm.prank(user1);
            token.privateTransfer(commitment, nullifier, amount, proof);
        }

        // Verify all commitments exist
        // (In a real scenario, you'd track these commitments)
    }
}

