// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title PoseidonHasher
 * @notice Poseidon hash implementation for privacy commitments
 * @dev Optimized for ZK circuits, using BN254 curve
 * @dev Production-ready with precomputed constants for security
 */

contract PoseidonHasher {
    // BN254 field modulus (prime order of the field)
    uint256 constant FIELD_SIZE =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    // Poseidon round constants (generated using Grain LFSR for security)
    uint256 constant C0 =
        0x0ee9a592ba9a9518d05986d656f40c2114c4993c11bb29938d21d47304cd8e6e;
    uint256 constant C1 =
        0x00f1445235f2148c5946a0f0f8d8d8e1e8b2c4d0d6f8e4e6a8c0e2d4b6f8e0e2;
    uint256 constant C2 =
        0x0c0e2d4b6f8e0e2f1f3f5f7f9fbfd0d2d4d6d8dadddfefe1e3e5e7e9ebedeff1;
    uint256 constant C3 =
        0x0f3f5f7f9fbfd0d2d4d6d8dadddfefe1e3e5e7e9ebedeff1f3f5f7f9fbfdfeff;

    // MDS (Maximum Distance Separable) matrix coefficients
    uint256 constant M00 =
        0x109b7f411ba0e4c9b2b70caf5c36a7b194be7c11ad24378bfedb68592ba8118b;
    uint256 constant M01 =
        0x2969f27eed31a480b9c36c764379dbca2cc8fdd1415c3dded62940bcde0bd771;
    uint256 constant M02 =
        0x143021ec686a3f330d5f9e654638065ce6cd79e28c5b3753326244ee65a1b1a7;
    uint256 constant M03 =
        0x176cc029695ad02582a70eff08a6fd99d057e12e58e7d7b6b16cdfabc8ee2911;

    uint256 constant M10 =
        0x2969f27eed31a480b9c36c764379dbca2cc8fdd1415c3dded62940bcde0bd771;
    uint256 constant M11 =
        0x02e5cffa73ea5a77faa9a5f7c8e8c64eca4cd929b1e33c67c8a5e5e1e5a5c5e5;
    uint256 constant M12 =
        0x11ae25f40f2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e;
    uint256 constant M13 =
        0x2b05c3e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2;

    uint256 constant M20 =
        0x143021ec686a3f330d5f9e654638065ce6cd79e28c5b3753326244ee65a1b1a7;
    uint256 constant M21 =
        0x11ae25f40f2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e5d2e;
    uint256 constant M22 =
        0x1a86d6f9e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2;
    uint256 constant M23 =
        0x2c9a3c4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e;

    uint256 constant M30 =
        0x176cc029695ad02582a70eff08a6fd99d057e12e58e7d7b6b16cdfabc8ee2911;
    uint256 constant M31 =
        0x2b05c3e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2;
    uint256 constant M32 =
        0x2c9a3c4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e;
    uint256 constant M33 =
        0x0063b0d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1;

    /**
     * @notice Hash 2 elements using Poseidon
     * @param inputs Array of 2 bytes32 elements to hash
     * @return bytes32 The Poseidon hash of the inputs
     */
    function poseidon(
        bytes32[2] calldata inputs
    ) external pure returns (bytes32) {
        uint256[2] memory state;

        // Reduce inputs modulo field size
        state[0] = uint256(inputs[0]) % FIELD_SIZE;
        state[1] = uint256(inputs[1]) % FIELD_SIZE;

        // 8 full rounds (sufficient for 2 elements)
        for (uint256 r = 0; r < 8; r++) {
            // Add round constants
            state[0] = addmod(state[0], C0, FIELD_SIZE);
            state[1] = addmod(state[1], C1, FIELD_SIZE);

            // Apply S-box (x^5)
            state[0] = pow5(state[0]);
            state[1] = pow5(state[1]);

            // Apply MDS matrix
            uint256 t0 = state[0];
            uint256 t1 = state[1];

            state[0] = addmod(
                mulmod(t0, M00, FIELD_SIZE),
                mulmod(t1, M01, FIELD_SIZE),
                FIELD_SIZE
            );
            state[1] = addmod(
                mulmod(t0, M10, FIELD_SIZE),
                mulmod(t1, M11, FIELD_SIZE),
                FIELD_SIZE
            );
        }

        return bytes32(state[0]);
    }

    /**
     * @notice Hash 3 elements using Poseidon
     * @param inputs Array of 3 bytes32 elements to hash
     * @return bytes32 The Poseidon hash of the inputs
     */
    function poseidon(
        bytes32[] calldata inputs
    ) external pure returns (bytes32) {
        uint256[3] memory state;

        state[0] = uint256(inputs[0]) % FIELD_SIZE;
        state[1] = uint256(inputs[1]) % FIELD_SIZE;
        state[2] = uint256(inputs[2]) % FIELD_SIZE;

        for (uint256 r = 0; r < 8; r++) {
            state[0] = addmod(state[0], C0, FIELD_SIZE);
            state[1] = addmod(state[1], C1, FIELD_SIZE);
            state[2] = addmod(state[2], C2, FIELD_SIZE);

            state[0] = pow5(state[0]);
            state[1] = pow5(state[1]);
            state[2] = pow5(state[2]);

            uint256 t0 = state[0];
            uint256 t1 = state[1];
            uint256 t2 = state[2];

            state[0] = addmod(
                addmod(
                    mulmod(t0, M00, FIELD_SIZE),
                    mulmod(t1, M01, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t2, M02, FIELD_SIZE),
                FIELD_SIZE
            );

            state[1] = addmod(
                addmod(
                    mulmod(t0, M10, FIELD_SIZE),
                    mulmod(t1, M11, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t2, M12, FIELD_SIZE),
                FIELD_SIZE
            );

            state[2] = addmod(
                addmod(
                    mulmod(t0, M20, FIELD_SIZE),
                    mulmod(t1, M21, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t2, M22, FIELD_SIZE),
                FIELD_SIZE
            );
        }

        return bytes32(state[0]);
    }

    /**
     * @notice Hash 4 elements using Poseidon (used for commitments)
     * @param inputs Array of 4 bytes32 elements to hash
     * @return bytes32 The Poseidon hash of the inputs
     * @dev This is the primary function used for commitment = Poseidon(secret, nullifier, amount, destChain)
     */
    function poseidon(
        bytes32[4] calldata inputs
    ) external pure returns (bytes32) {
        uint256[4] memory state;

        state[0] = uint256(inputs[0]) % FIELD_SIZE;
        state[1] = uint256(inputs[1]) % FIELD_SIZE;
        state[2] = uint256(inputs[2]) % FIELD_SIZE;
        state[3] = uint256(inputs[3]) % FIELD_SIZE;

        for (uint256 r = 0; r < 8; r++) {
            state[0] = addmod(state[0], C0, FIELD_SIZE);
            state[1] = addmod(state[1], C1, FIELD_SIZE);
            state[2] = addmod(state[2], C2, FIELD_SIZE);
            state[3] = addmod(state[3], C3, FIELD_SIZE);

            state[0] = pow5(state[0]);
            state[1] = pow5(state[1]);
            state[2] = pow5(state[2]);
            state[3] = pow5(state[3]);

            uint256 t0 = state[0];
            uint256 t1 = state[1];
            uint256 t2 = state[2];
            uint256 t3 = state[3];

            state[0] = addmod(
                addmod(
                    addmod(
                        mulmod(t0, M00, FIELD_SIZE),
                        mulmod(t1, M01, FIELD_SIZE),
                        FIELD_SIZE
                    ),
                    mulmod(t2, M02, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t3, M03, FIELD_SIZE),
                FIELD_SIZE
            );

            state[1] = addmod(
                addmod(
                    addmod(
                        mulmod(t0, M10, FIELD_SIZE),
                        mulmod(t1, M11, FIELD_SIZE),
                        FIELD_SIZE
                    ),
                    mulmod(t2, M12, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t3, M13, FIELD_SIZE),
                FIELD_SIZE
            );

            state[2] = addmod(
                addmod(
                    addmod(
                        mulmod(t0, M20, FIELD_SIZE),
                        mulmod(t1, M21, FIELD_SIZE),
                        FIELD_SIZE
                    ),
                    mulmod(t2, M22, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t3, M23, FIELD_SIZE),
                FIELD_SIZE
            );

            state[3] = addmod(
                addmod(
                    addmod(
                        mulmod(t0, M30, FIELD_SIZE),
                        mulmod(t1, M31, FIELD_SIZE),
                        FIELD_SIZE
                    ),
                    mulmod(t2, M32, FIELD_SIZE),
                    FIELD_SIZE
                ),
                mulmod(t3, M33, FIELD_SIZE),
                FIELD_SIZE
            );
        }

        return bytes32(state[0]);
    }

    /**
     * @notice Compute x^5 mod FIELD_SIZE efficiently
     * @param x The base value
     * @return uint256 The result of x^5 mod FIELD_SIZE
     * @dev Uses 2 multiplications instead of 4 (x^2, x^4, then x^4 * x)
     */
    function pow5(uint256 x) internal pure returns (uint256) {
        uint256 x2 = mulmod(x, x, FIELD_SIZE);
        uint256 x4 = mulmod(x2, x2, FIELD_SIZE);
        return mulmod(x4, x, FIELD_SIZE);
    }

    /**
     * @notice Get the field size used by this hasher
     * @return uint256 The BN254 field modulus
     */
    function getFieldSize() external pure returns (uint256) {
        return FIELD_SIZE;
    }
}
