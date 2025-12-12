// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {MockToken} from "../src/MockToken.sol";

/**
 * @title DeployTestTokens
 * @notice Deploy test tokens on both chains
 */
contract DeployTestTokens is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        
        console.log("=== Deploying Test Tokens ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Deploy USDC (6 decimals, 1M supply)
        MockToken usdc = new MockToken(
            "USD Coin",
            "USDC",
            6,
            1_000_000 * 10**6
        );
        console.log("USDC deployed at:", address(usdc));
        
        // Deploy USDT (6 decimals, 1M supply)
        MockToken usdt = new MockToken(
            "Tether USD",
            "USDT",
            6,
            1_000_000 * 10**6
        );
        console.log("USDT deployed at:", address(usdt));
        
        // Deploy WETH (18 decimals, 10K supply)
        MockToken weth = new MockToken(
            "Wrapped Ether",
            "WETH",
            18,
            10_000 * 10**18
        );
        console.log("WETH deployed at:", address(weth));
        
        vm.stopBroadcast();
        
        console.log("");
        console.log("=== DEPLOYMENT SUMMARY ===");
        console.log("USDC:", address(usdc));
        console.log("USDT:", address(usdt));
        console.log("WETH:", address(weth));
        console.log("");
        console.log("Update your token addresses in relayer config!");
    }
}

/**
 * @title DeployMantleTokens
 * @notice Deploy only MNT token for Mantle
 */
contract DeployMantleTokens is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        console.log("=== Deploying Mantle Test Tokens ===");
        console.log("Chain ID:", block.chainid);
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Deploy MNT (18 decimals, 10M supply)
        MockToken mnt = new MockToken(
            "Mantle",
            "MNT",
            18,
            10_000_000 * 10**18
        );
        console.log("MNT deployed at:", address(mnt));
        
        // Deploy USDC (6 decimals, 1M supply)
        MockToken usdc = new MockToken(
            "USD Coin",
            "USDC",
            6,
            1_000_000 * 10**6
        );
        console.log("USDC deployed at:", address(usdc));
        
        // Deploy USDT (6 decimals, 1M supply)
        MockToken usdt = new MockToken(
            "Tether USD",
            "USDT",
            6,
            1_000_000 * 10**6
        );
        console.log("USDT deployed at:", address(usdt));
        
        vm.stopBroadcast();
    }
}
/*
forge script script/DeployTokens.s.sol:DeployMantleTokens \
  --rpc-url $MANTLE_RPC_URL \
  --broadcast \
  --verify \
  -vvvv

  forge script script/DeployTokens.s.sol:DeployTestTokens \
  --rpc-url $ETHEREUM_RPC_URL \
  --broadcast \
  --verify \
  -vvvv
  */