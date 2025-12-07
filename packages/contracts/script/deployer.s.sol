// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";

/**
 * @title DeployPrivacyBridge
 * @notice Deployment script for Privacy Bridge contracts
 * @dev Usage: forge script script/Deploy.s.sol:DeployPrivacyBridge --rpc-url <RPC_URL> --broadcast
 */
contract DeployPrivacyBridge is Script {
    
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR");
        
        console.log("=== Privacy Bridge Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Relayer:", relayer);
        console.log("Fee Collector:", feeCollector);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // 1. Deploy Poseidon Hasher
        console.log("Deploying PoseidonHasher...");
        PoseidonHasher poseidon = new PoseidonHasher();
        console.log("PoseidonHasher deployed at:", address(poseidon));
        console.log("");
        
        // 2. Deploy PrivateIntentPool (Source chain - Mantle)
        console.log("Deploying PrivateIntentPool...");
        PrivateIntentPool intentPool = new PrivateIntentPool(
            relayer,
            feeCollector,
            address(poseidon)
        );
        console.log("PrivateIntentPool deployed at:", address(intentPool));
        console.log("");
        
        // 3. Deploy PrivateSettlement (Destination chain - Ethereum)
        console.log("Deploying PrivateSettlement...");
        PrivateSettlement settlement = new PrivateSettlement(
            relayer,
            feeCollector,
            address(poseidon)
        );
        console.log("PrivateSettlement deployed at:", address(settlement));
        console.log("");
        
        vm.stopBroadcast();
        
        // Save deployment info
        console.log("=== DEPLOYMENT SUMMARY ===");
        console.log("Network:", block.chainid);
        console.log("");
        console.log("Contracts:");
        console.log("  PoseidonHasher:", address(poseidon));
        console.log("  PrivateIntentPool:", address(intentPool));
        console.log("  PrivateSettlement:", address(settlement));
        console.log("");
        console.log("Configuration:");
        console.log("  Relayer:", relayer);
        console.log("  FeeCollector:", feeCollector);
        console.log("  Intent Fee:", "0.1% (10 bps)");
        console.log("  Settlement Fee:", "0.05% (5 bps)");
        console.log("");
        console.log("Save these addresses for relayer configuration!");
    }
}

/**
 * @title DeployMantleOnly
 * @notice Deploy only source chain contracts (Mantle)
 */
contract DeployMantleOnly is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR");
        
        console.log("=== Mantle Source Chain Deployment ===");
        
        vm.startBroadcast(deployerPrivateKey);
        
        PoseidonHasher poseidon = new PoseidonHasher();
        console.log("PoseidonHasher:", address(poseidon));
        
        PrivateIntentPool intentPool = new PrivateIntentPool(
            relayer,
            feeCollector,
            address(poseidon)
        );
        console.log("PrivateIntentPool:", address(intentPool));
        
        vm.stopBroadcast();
    }
}

/**
 * @title DeployEthereumOnly
 * @notice Deploy only destination chain contracts (Ethereum)
 */
contract DeployEthereumOnly is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR");
        
        console.log("=== Ethereum Destination Chain Deployment ===");
        
        vm.startBroadcast(deployerPrivateKey);
        
        PoseidonHasher poseidon = new PoseidonHasher();
        console.log("PoseidonHasher:", address(poseidon));
        
        PrivateSettlement settlement = new PrivateSettlement(
            relayer,
            feeCollector,
            address(poseidon)
        );
        console.log("PrivateSettlement:", address(settlement));
        
        vm.stopBroadcast();
    }
}