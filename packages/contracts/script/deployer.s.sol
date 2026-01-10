// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {PrivateIntentPool} from "../src/PrivateIntentPool.sol";
import {PrivateSettlement} from "../src/PrivateSettlement.sol";

/** 
 * @title DeployPoseidonHasher
 * @notice Deploy PoseidonHasher on ANY chain
 * @dev Usage (Mantle): forge script script/Deployer.s.sol:DeployPoseidonHasher --rpc-url $MANTLE_RPC_URL --broadcast --verify
 * @dev Usage (Ethereum): forge script script/Deployer.s.sol:DeployPoseidonHasher --rpc-url $ETHEREUM_RPC_URL --broadcast --verify
 * @dev IMPORTANT: Run this on BOTH chains and save each address separately
 */
contract DeployPoseidonHasher is Script {
    function run() external returns (address) {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        console.log("=== PoseidonHasher Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Chain ID:", block.chainid);
        
        string memory chainName = block.chainid == 5003 ? "Mantle Sepolia" : 
                                  block.chainid == 5000 ? "Mantle" :
                                  block.chainid == 11155111 ? "Ethereum Sepolia" :
                                  block.chainid == 1 ? "Ethereum Mainnet" : "Unknown";
        console.log("Chain:", chainName);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        PoseidonHasher poseidon = new PoseidonHasher();
        console.log("PoseidonHasher deployed at:", address(poseidon));
        
        vm.stopBroadcast();
        
        console.log("");
        console.log("=== SAVE THIS ADDRESS ===");
        if (block.chainid == 5003 || block.chainid == 5000) {
            console.log("Add to .env: MANTLE_POSEIDON_HASHER_ADDRESS=", address(poseidon));
        } else {
            console.log("Add to .env: ETHEREUM_POSEIDON_HASHER_ADDRESS=", address(poseidon));
        }
        
        return address(poseidon);
    }
} 

/**
 * @title DeployMantleContracts
 * @notice Deploy PrivateIntentPool and PrivateSettlement on Mantle
 * @dev Usage: forge script script/Deployer.s.sol:DeployMantleContracts --rpc-url $MANTLE_RPC_URL --broadcast --verify
 * @dev REQUIRES: MANTLE_POSEIDON_HASHER_ADDRESS in .env
 */
contract DeployMantleContracts is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR_ADDRESS");
        address poseidonHasher = vm.envAddress("MANTLE_POSEIDON_HASHER_ADDRESS");
        address owner = vm.envAddress("OWNER_ADDRESS");
        
        console.log("=== Mantle Chain Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Owner:", owner);
        console.log("Relayer:", relayer);
        console.log("Fee Collector:", feeCollector);
        console.log("Poseidon Hasher (Mantle):", poseidonHasher);
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        require(poseidonHasher != address(0), "MANTLE_POSEIDON_HASHER_ADDRESS not set");
        
        vm.startBroadcast(deployerPrivateKey);
        
        console.log("Deploying PrivateIntentPool...");
        PrivateIntentPool intentPool = new PrivateIntentPool(
            owner,
            relayer,
            feeCollector,
            poseidonHasher
        );
        console.log("PrivateIntentPool deployed at:", address(intentPool));
        console.log("");
        
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
        
        console.log("=== MANTLE DEPLOYMENT COMPLETE ===");
        console.log("PrivateIntentPool:", address(intentPool));
        console.log("PrivateSettlement:", address(settlement));
        console.log("");
        console.log("Add to .env:");
        console.log("MANTLE_INTENT_POOL_ADDRESS=", address(intentPool));
        console.log("MANTLE_SETTLEMENT_ADDRESS=", address(settlement));
    }
}

/**
 * @title DeployEthereumContracts
 * @notice Deploy PrivateIntentPool and PrivateSettlement on Ethereum
 * @dev Usage: forge script script/Deployer.s.sol:DeployEthereumContracts --rpc-url $ETHEREUM_RPC_URL --broadcast --verify
 * @dev REQUIRES: ETHEREUM_POSEIDON_HASHER_ADDRESS in .env
 */
contract DeployEthereumContracts is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        address feeCollector = vm.envAddress("FEE_COLLECTOR_ADDRESS");
        address poseidonHasher = vm.envAddress("ETHEREUM_POSEIDON_HASHER_ADDRESS");
        address owner = vm.envAddress("OWNER_ADDRESS");
        
        console.log("=== Ethereum Chain Deployment ===");
        console.log("Deployer:", vm.addr(deployerPrivateKey));
        console.log("Owner:", owner);
        console.log("Relayer:", relayer);
        console.log("Fee Collector:", feeCollector);
        console.log("Poseidon Hasher (Ethereum):", poseidonHasher);
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        require(poseidonHasher != address(0), "ETHEREUM_POSEIDON_HASHER_ADDRESS not set");
        
        vm.startBroadcast(deployerPrivateKey);
        
        console.log("Deploying PrivateIntentPool...");
        PrivateIntentPool intentPool = new PrivateIntentPool(
            owner,
            relayer,
            feeCollector,
            poseidonHasher
        );
        console.log("PrivateIntentPool deployed at:", address(intentPool));
        console.log("");
        
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
        
        console.log("=== ETHEREUM DEPLOYMENT COMPLETE ===");
        console.log("PrivateIntentPool:", address(intentPool));
        console.log("PrivateSettlement:", address(settlement));
        console.log("");
        console.log("Add to .env:");
        console.log("ETHEREUM_INTENT_POOL_ADDRESS=", address(intentPool));
        console.log("ETHEREUM_SETTLEMENT_ADDRESS=", address(settlement));
    }
}

/**
 * @title ConfigureTokens
 * @notice Configure supported tokens on both IntentPool and Settlement
 * @dev Usage (Mantle): forge script script/Deployer.s.sol:ConfigureTokens --rpc-url $MANTLE_RPC_URL --broadcast
 * @dev Usage (Ethereum): forge script script/Deployer.s.sol:ConfigureTokens --rpc-url $ETHEREUM_RPC_URL --broadcast
 */
 /*
contract ConfigureTokens is Script {
    address constant NATIVE_ETH = address(0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE);
    
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        address intentPool;
        address settlement;
        
        if (block.chainid == 5003 || block.chainid == 5000) {
            console.log("=== Configuring Mantle Tokens ===");
            intentPool = vm.envAddress("MANTLE_INTENT_POOL_ADDRESS");
            settlement = vm.envAddress("MANTLE_SETTLEMENT_ADDRESS");
        } else {
            console.log("=== Configuring Ethereum Tokens ===");
            intentPool = vm.envAddress("ETHEREUM_INTENT_POOL_ADDRESS");
            settlement = vm.envAddress("ETHEREUM_SETTLEMENT_ADDRESS");
        }
        
        console.log("IntentPool:", intentPool);
        console.log("Settlement:", settlement);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        console.log("Adding ETH support (min: 0.01 ETH, max: 100 ETH)...");
        PrivateIntentPool(intentPool).addSupportedToken(NATIVE_ETH, 0.01 ether, 100 ether, 18);
        PrivateSettlement(settlement).addSupportedToken(NATIVE_ETH, 0.01 ether, 100 ether, 18);
        console.log("ETH configured!");
        console.log("");
        
        vm.stopBroadcast();
        
        console.log("=== TOKEN CONFIGURATION COMPLETE ===");
        console.log("Supported tokens:");
        console.log("  - Native ETH (0.01 - 100 ETH)");
        console.log("");
        console.log("To add more tokens, call addSupportedToken() on both contracts");
    }
}

*/