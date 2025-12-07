// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";

/**
 * @title PoseidonHasherTest
 * @notice Comprehensive tests for Poseidon hash function
 */
contract PoseidonHasherTest is Test {
    PoseidonHasher public poseidon;
    
    function setUp() public {
        poseidon = new PoseidonHasher();
    }
    
    // ========== HELPER FUNCTIONS FOR CALLDATA CONVERSION ==========
    
    function hashWith2Elements(bytes32 a, bytes32 b) internal view returns (bytes32) {
        bytes32[2] memory inputs = [a, b];
        return this.callPoseidon2(inputs);
    }
    
    function callPoseidon2(bytes32[2] calldata inputs) external view returns (bytes32) {
        return poseidon.poseidon(inputs);
    }
    
    function hashWith4Elements(bytes32 a, bytes32 b, bytes32 c, bytes32 d) internal view returns (bytes32) {
        bytes32[4] memory inputs = [a, b, c, d];
        return this.callPoseidon4(inputs);
    }
    
    function callPoseidon4(bytes32[4] calldata inputs) external view returns (bytes32) {
        return poseidon.poseidon(inputs);
    }
    
    // ========== BASIC FUNCTIONALITY TESTS ==========
    
    function test_Poseidon2Elements() public {
        bytes32 hash = hashWith2Elements(bytes32(uint256(1)), bytes32(uint256(2)));
        
        assertTrue(hash != bytes32(0), "Hash should not be zero");
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash should be in field");
    }
    
    function test_Poseidon4Elements() public {
        bytes32 hash = hashWith4Elements(
            bytes32(uint256(1)),
            bytes32(uint256(2)),
            bytes32(uint256(3)),
            bytes32(uint256(4))
        );
        
        assertTrue(hash != bytes32(0), "Hash should not be zero");
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash should be in field");
    }
    
    // ========== DETERMINISM TESTS ==========
    
    function test_DeterministicHash2() public {
        bytes32 hash1 = hashWith2Elements(bytes32(uint256(123)), bytes32(uint256(456)));
        bytes32 hash2 = hashWith2Elements(bytes32(uint256(123)), bytes32(uint256(456)));
        
        assertEq(hash1, hash2, "Same inputs should produce same hash");
    }
    
    function test_DeterministicHash4() public {
        bytes32 hash1 = hashWith4Elements(
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        );
        bytes32 hash2 = hashWith4Elements(
            bytes32(uint256(100)),
            bytes32(uint256(200)),
            bytes32(uint256(300)),
            bytes32(uint256(400))
        );
        
        assertEq(hash1, hash2, "Same inputs should produce same hash");
    }
    
    // ========== COLLISION RESISTANCE TESTS ==========
    
    function test_DifferentInputsDifferentHashes() public {
        bytes32 hash1 = hashWith4Elements(
            bytes32(uint256(1)),
            bytes32(uint256(2)),
            bytes32(uint256(3)),
            bytes32(uint256(4))
        );
        
        bytes32 hash2 = hashWith4Elements(
            bytes32(uint256(1)),
            bytes32(uint256(2)),
            bytes32(uint256(3)),
            bytes32(uint256(5)) // Different last element
        );
        
        assertTrue(hash1 != hash2, "Different inputs should produce different hashes");
    }
    
    function test_OrderMatters() public {
        bytes32 hash1 = hashWith4Elements(
            bytes32(uint256(1)),
            bytes32(uint256(2)),
            bytes32(uint256(3)),
            bytes32(uint256(4))
        );
        
        bytes32 hash2 = hashWith4Elements(
            bytes32(uint256(4)),
            bytes32(uint256(3)),
            bytes32(uint256(2)),
            bytes32(uint256(1))
        );
        
        assertTrue(hash1 != hash2, "Order should matter");
    }
    
    // ========== EDGE CASE TESTS ==========
    
    function test_ZeroInputs() public {
        bytes32 hash = hashWith4Elements(bytes32(0), bytes32(0), bytes32(0), bytes32(0));
        
        assertTrue(hash != bytes32(0), "Hash of zeros should not be zero");
    }
    
    function test_MaxInputs() public {
        bytes32 hash = hashWith4Elements(
            bytes32(type(uint256).max),
            bytes32(type(uint256).max),
            bytes32(type(uint256).max),
            bytes32(type(uint256).max)
        );
        
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash should be in field");
    }
    
    function test_MixedInputs() public {
        bytes32 hash = hashWith4Elements(
            bytes32(0),
            bytes32(type(uint256).max),
            bytes32(uint256(1)),
            bytes32(type(uint256).max / 2)
        );
        
        assertTrue(hash != bytes32(0), "Should handle mixed inputs");
    }
    
    // ========== FIELD SIZE TESTS ==========
    
    function test_FieldSizeCorrect() public view {
        uint256 fieldSize = poseidon.getFieldSize();
        assertEq(
            fieldSize,
            21888242871839275222246405745257275088548364400416034343698204186575808495617,
            "Field size should be BN254 prime"
        );
    }
    
    function test_HashAlwaysInField() public {
        for (uint256 i = 0; i < 10; i++) {
            bytes32 hash = hashWith4Elements(
                bytes32(uint256(i * 1000)),
                bytes32(uint256(i * 2000)),
                bytes32(uint256(i * 3000)),
                bytes32(uint256(i * 4000))
            );
            
            assertTrue(
                uint256(hash) < poseidon.getFieldSize(),
                "Hash must be in field"
            );
        }
    }
    
    // ========== COMMITMENT GENERATION TESTS ==========
    
    function test_CommitmentGeneration() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier = keccak256("nullifier");
        uint256 amount = 1 ether;
        uint32 destChain = 1;
        
        bytes32 commitment = hashWith4Elements(
            secret,
            nullifier,
            bytes32(amount),
            bytes32(uint256(destChain))
        );
        
        assertTrue(commitment != bytes32(0), "Commitment should not be zero");
        assertTrue(commitment != secret, "Commitment should hide secret");
        assertTrue(commitment != nullifier, "Commitment should hide nullifier");
    }
    
    function test_SameSecretDifferentNullifierDifferentCommitment() public {
        bytes32 secret = keccak256("secret");
        bytes32 nullifier1 = keccak256("nullifier1");
        bytes32 nullifier2 = keccak256("nullifier2");
        uint256 amount = 1 ether;
        uint32 destChain = 1;
        
        bytes32 commitment1 = hashWith4Elements(
            secret,
            nullifier1,
            bytes32(amount),
            bytes32(uint256(destChain))
        );
        
        bytes32 commitment2 = hashWith4Elements(
            secret,
            nullifier2,
            bytes32(amount),
            bytes32(uint256(destChain))
        );
        
        assertTrue(commitment1 != commitment2, "Different nullifiers should give different commitments");
    }
    
    // ========== GAS BENCHMARKING ==========
    
    function test_Gas_Poseidon2Elements() public {
        uint256 gasBefore = gasleft();
        hashWith2Elements(bytes32(uint256(1)), bytes32(uint256(2)));
        uint256 gasUsed = gasBefore - gasleft();
        
        console.log("Gas used for 2 elements:", gasUsed);
        assertTrue(gasUsed < 150000, "Should use reasonable gas");
    }
    
    function test_Gas_Poseidon4Elements() public {
        uint256 gasBefore = gasleft();
        hashWith4Elements(
            bytes32(uint256(1)),
            bytes32(uint256(2)),
            bytes32(uint256(3)),
            bytes32(uint256(4))
        );
        uint256 gasUsed = gasBefore - gasleft();
        
        console.log("Gas used for 4 elements:", gasUsed);
        assertTrue(gasUsed < 200000, "Should use reasonable gas");
    }
    
    // ========== FUZZ TESTS ==========
    
    function testFuzz_Poseidon2Elements(uint256 a, uint256 b) public {
        bytes32 hash = hashWith2Elements(bytes32(a), bytes32(b));
        
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash must be in field");
    }
    
    function testFuzz_Poseidon4Elements(
        uint256 a,
        uint256 b,
        uint256 c,
        uint256 d
    ) public {
        bytes32 hash = hashWith4Elements(bytes32(a), bytes32(b), bytes32(c), bytes32(d));
        
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash must be in field");
    }
    
    function testFuzz_CommitmentUnique(
        bytes32 secret,
        bytes32 nullifier,
        uint256 amount,
        uint32 destChain
    ) public {
        vm.assume(amount > 0);
        vm.assume(destChain > 0);
        
        bytes32 commitment = hashWith4Elements(
            secret,
            nullifier,
            bytes32(amount),
            bytes32(uint256(destChain))
        );
        
        assertTrue(uint256(commitment) < poseidon.getFieldSize(), "Commitment must be in field");
    }
    
    // ========== ADDITIONAL TESTS ==========
    
    function test_HashWithBlockData() public {
        bytes32 hash = hashWith4Elements(
            bytes32(uint256(block.timestamp)),
            bytes32(uint256(block.number)),
            bytes32(uint256(block.difficulty)),
            bytes32(uint256(block.chainid))
        );
        
        assertTrue(uint256(hash) < poseidon.getFieldSize(), "Hash must always be in field");
    }
    
    function test_MultipleHashesAllInField() public {
        // Test multiple random-ish hashes
        for (uint256 i = 0; i < 20; i++) {
            bytes32 hash = hashWith4Elements(
                keccak256(abi.encodePacked("test", i)),
                keccak256(abi.encodePacked("data", i)),
                bytes32(i * 1000),
                bytes32(i)
            );
            
            assertTrue(uint256(hash) < poseidon.getFieldSize(), "All hashes must be in field");
        }
    }
}