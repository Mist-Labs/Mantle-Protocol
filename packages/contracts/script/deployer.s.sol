// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";

/**
 * @title DeployPoseidonHasher
 * @notice Deploy PoseidonHasher on Mantle (cheaper gas)
 * @dev Usage: forge script script/Deployer.s.sol:DeployPoseidonHasher --rpc-url $MANTLE_RPC_URL --broadcast --verify
 
contract DeployPoseidonHasher is Script {
    function run() external returns (address) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        console.log("=== PoseidonHasher Deployment (Mantle) ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        PoseidonHasher poseidon = new PoseidonHasher();
        console.log("PoseidonHasher deployed at:", address(poseidon));
        
        vm.stopBroadcast();
        
        console.log("");
        console.log("SAVE THIS ADDRESS! You'll need it for both chains.");
        console.log("Add to .env: POSEIDON_HASHER_ADDRESS=", address(poseidon));
        
        return address(poseidon);
    }
} */

/**
 * @title DeployMantleContracts
 * @notice Deploy PrivateIntentPool and PrivateSettlement on Mantle
 * @dev Usage: forge script script/Deployer.s.sol:DeployMantleContracts --rpc-url $MANTLE_RPC_URL --broadcast --verify
 */
contract DeployMantleContracts is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR_ADDRESS");
        address poseidonHasher = vm.envAddress("POSEIDON_HASHER_ADDRESS");
        address owner = vm.envAddress("OWNER_ADDRESS");
        
        console.log("=== Mantle Chain Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Relayer:", relayer);
        console.log("Owner:", owner);
        console.log("Fee Collector:", feeCollector);
        console.log("Poseidon Hasher:", poseidonHasher);
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Deploy PrivateIntentPool
        // console.log("Deploying PrivateIntentPool...");
        // PrivateIntentPool intentPool = new PrivateIntentPool(
        //     owner,
        //     relayer,
        //     feeCollector,
        //     poseidonHasher
        // );
        // console.log("PrivateIntentPool deployed at:", address(intentPool));
        // console.log("");
        
        // Deploy PrivateSettlement
        console.log("Deploying PrivateSettlement...");
        PrivateSettlement settlement = new PrivateSettlement(
            owner,
            relayer,
            feeCollector,
            poseidonHasher
        );
        console.log("PrivateSettlement deployed at:", address(settlement));
        console.log("");
        
        vm.stopBroadcast();
        
        console.log("=== MANTLE DEPLOYMENT SUMMARY ===");
        // console.log("PrivateIntentPool:", address(intentPool));
        console.log("PrivateSettlement:", address(settlement));
        console.log("");
        console.log("Add to .env:");
        // console.log("MANTLE_INTENT_POOL_ADDRESS=", address(intentPool));
        console.log("MANTLE_SETTLEMENT_ADDRESS=", address(settlement));
    }
}

/**
 * @title DeployEthereumContracts
 * @notice Deploy PrivateIntentPool and PrivateSettlement on Ethereum
 * @dev Usage: forge script script/Deployer.s.sol:DeployEthereumContracts --rpc-url $ETHEREUM_RPC_URL --broadcast --verify
 */
contract DeployEthereumContracts is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR_ADDRESS");
        address poseidonHasher = vm.envAddress("POSEIDON_HASHER_ADDRESS");
        address owner = vm.envAddress("OWNER_ADDRESS");
        
        console.log("=== Ethereum Chain Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Relayer:", relayer);
        console.log("Owner:", owner);
        console.log("Fee Collector:", feeCollector);
        console.log("Poseidon Hasher:", poseidonHasher);
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Deploy PrivateIntentPool
        // console.log("Deploying PrivateIntentPool...");
        // PrivateIntentPool intentPool = new PrivateIntentPool(
        //     owner,
        //     relayer,
        //     feeCollector,
        //     poseidonHasher
        // );
        // console.log("PrivateIntentPool deployed at:", address(intentPool));
        // console.log("");
        
        // Deploy PrivateSettlement
        console.log("Deploying PrivateSettlement...");
        PrivateSettlement settlement = new PrivateSettlement(
            owner,
            relayer,
            feeCollector,
            poseidonHasher
        );
        console.log("PrivateSettlement deployed at:", address(settlement));
        console.log("");
        
        vm.stopBroadcast();
        
        console.log("=== ETHEREUM DEPLOYMENT SUMMARY ===");
        // console.log("PrivateIntentPool:", address(intentPool));
        console.log("PrivateSettlement:", address(settlement));
        console.log("");
        console.log("Add to .env:");
        // console.log("ETHEREUM_INTENT_POOL_ADDRESS=", address(intentPool));
        console.log("ETHEREUM_SETTLEMENT_ADDRESS=", address(settlement));
    }
}