interface IPoseidonHasher {
    function poseidon(bytes32[2] calldata inputs) external pure returns (bytes32);
    function poseidon(bytes32[3] calldata inputs) external pure returns (bytes32);
    function poseidon(bytes32[4] calldata inputs) external pure returns (bytes32);
     function getFieldSize() external pure returns (uint256);
}

