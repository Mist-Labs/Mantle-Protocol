// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.20;

import "../PoseidonHasher.sol";

/**
 * @title Verifier
 * @notice Basic commitment verifier using Poseidon hashing
 * @dev For MVP: Simple Poseidon hash verification (not full zkSNARKs)
 * @dev Uses the existing PoseidonHasher contract for gas-efficient hashing
 */
contract Verifier {
    PoseidonHasher public immutable hasher;

    /**
     * @notice Initialize the verifier with a PoseidonHasher contract
     * @param _hasher The address of the deployed PoseidonHasher contract
     */
    constructor(address _hasher) {
        require(_hasher != address(0), "Invalid hasher address");
        hasher = PoseidonHasher(_hasher);
    }

    /**
     * @notice Verify a commitment hash
     * @dev For MVP: commitment = Poseidon(amount, blinding, recipient, nonce)
     * @param inputs Array of 4 bytes32 inputs: [amount, blinding, recipient, nonce]
     * @return commitment The computed commitment hash
     */
    function verifyCommitment(
        bytes32[4] calldata inputs
    ) external view returns (bytes32 commitment) {
        // Convert bytes32[4] to the format expected by PoseidonHasher
        // PoseidonHasher.poseidon expects bytes32[4] calldata
        return hasher.poseidon(inputs);
    }

    /**
     * @notice Verify a commitment with 3 inputs (alternative format)
     * @param inputs Array of 3 bytes32 inputs
     * @return commitment The computed commitment hash
     */
    function verifyCommitment3(
        bytes32[3] calldata inputs
    ) external view returns (bytes32 commitment) {
        // Convert to bytes32[] for the overloaded poseidon function
        bytes32[] memory inputArray = new bytes32[](3);
        inputArray[0] = inputs[0];
        inputArray[1] = inputs[1];
        inputArray[2] = inputs[2];
        return hasher.poseidon(inputArray);
    }

    /**
     * @notice Verify a commitment with 2 inputs (simplified format)
     * @param inputs Array of 2 bytes32 inputs
     * @return commitment The computed commitment hash
     */
    function verifyCommitment2(
        bytes32[2] calldata inputs
    ) external view returns (bytes32 commitment) {
        return hasher.poseidon(inputs);
    }

    /**
     * @notice Check if a commitment matches expected hash
     * @param inputs Array of 4 bytes32 inputs
     * @param expectedCommitment The expected commitment hash
     * @return True if commitment matches
     */
    function verifyCommitmentMatch(
        bytes32[4] calldata inputs,
        bytes32 expectedCommitment
    ) external view returns (bool) {
        bytes32 computed = hasher.poseidon(inputs);
        return computed == expectedCommitment;
    }
}

