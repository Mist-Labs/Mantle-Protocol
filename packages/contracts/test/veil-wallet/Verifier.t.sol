// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {Verifier} from "../../src/veil-wallet/Verifier.sol";
import {PoseidonHasher} from "../../src/PoseidonHasher.sol";

/**
 * @title VerifierTest
 * @notice Comprehensive test suite for Verifier contract
 */
contract VerifierTest is Test {
    Verifier public verifier;
    PoseidonHasher public hasher;

    function setUp() public {
        hasher = new PoseidonHasher();
        verifier = new Verifier(address(hasher));
    }

    function test_Constructor_SetsHasher() public {
        assertEq(address(verifier.hasher()), address(hasher));
    }

    function test_Constructor_RevertsOnZeroAddress() public {
        vm.expectRevert("Invalid hasher address");
        new Verifier(address(0));
    }

    function test_VerifyCommitment_4Inputs() public {
        bytes32[4] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        ];
        
        bytes32 result = verifier.verifyCommitment(inputs);
        bytes32 expected = hasher.poseidon(inputs);
        
        assertEq(result, expected);
    }

    function test_VerifyCommitment_3Inputs() public {
        bytes32[3] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300))
        ];
        
        bytes32 result = verifier.verifyCommitment3(inputs);
        
        // Convert to array for hasher
        bytes32[] memory inputArray = new bytes32[](3);
        inputArray[0] = inputs[0];
        inputArray[1] = inputs[1];
        inputArray[2] = inputs[2];
        bytes32 expected = hasher.poseidon(inputArray);
        
        assertEq(result, expected);
    }

    function test_VerifyCommitment_2Inputs() public {
        bytes32[2] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200))
        ];
        
        bytes32 result = verifier.verifyCommitment2(inputs);
        bytes32 expected = hasher.poseidon(inputs);
        
        assertEq(result, expected);
    }

    function test_VerifyCommitmentMatch_ReturnsTrue() public {
        bytes32[4] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        ];
        
        bytes32 expectedCommitment = hasher.poseidon(inputs);
        bool result = verifier.verifyCommitmentMatch(inputs, expectedCommitment);
        
        assertTrue(result);
    }

    function test_VerifyCommitmentMatch_ReturnsFalse() public {
        bytes32[4] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        ];
        
        bytes32 wrongCommitment = bytes32(uint256(999));
        bool result = verifier.verifyCommitmentMatch(inputs, wrongCommitment);
        
        assertFalse(result);
    }

    function test_VerifyCommitment_DifferentInputs_DifferentOutputs() public {
        bytes32[4] memory inputs1 = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        ];
        
        bytes32[4] memory inputs2 = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(401)) // Different last input
        ];
        
        bytes32 result1 = verifier.verifyCommitment(inputs1);
        bytes32 result2 = verifier.verifyCommitment(inputs2);
        
        assertNotEq(result1, result2);
    }

    function test_VerifyCommitment_Deterministic() public {
        bytes32[4] memory inputs = [
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        ];
        
        bytes32 result1 = verifier.verifyCommitment(inputs);
        bytes32 result2 = verifier.verifyCommitment(inputs);
        
        assertEq(result1, result2);
    }

    function test_VerifyCommitment_ZeroInputs() public {
        bytes32[4] memory inputs = [
            bytes32(0),
            bytes32(0),
            bytes32(0),
            bytes32(0)
        ];
        
        bytes32 result = verifier.verifyCommitment(inputs);
        bytes32 expected = hasher.poseidon(inputs);
        
        assertEq(result, expected);
    }

    function test_VerifyCommitment_MaxInputs() public {
        bytes32[4] memory inputs = [
            bytes32(type(uint256).max),
            bytes32(type(uint256).max),
            bytes32(type(uint256).max),
            bytes32(type(uint256).max)
        ];
        
        bytes32 result = verifier.verifyCommitment(inputs);
        bytes32 expected = hasher.poseidon(inputs);
        
        assertEq(result, expected);
    }

    function test_VerifyCommitment_RealWorldScenario() public {
        // Simulate a real commitment: amount, blinding, recipient, nonce
        uint256 amount = 1 ether;
        bytes32 blinding = keccak256("secret_blinding");
        bytes32 recipient = bytes32(uint256(uint160(makeAddr("recipient"))));
        bytes32 nonce = bytes32(uint256(12345));
        
        bytes32[4] memory inputs = [bytes32(amount), blinding, recipient, nonce];
        
        bytes32 commitment = verifier.verifyCommitment(inputs);
        
        // Verify it matches direct hasher call
        bytes32 expected = hasher.poseidon(inputs);
        assertEq(commitment, expected);
        
        // Verify match function works
        assertTrue(verifier.verifyCommitmentMatch(inputs, commitment));
    }
}

