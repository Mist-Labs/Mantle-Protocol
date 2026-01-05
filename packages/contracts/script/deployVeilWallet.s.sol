// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {PoseidonHasher} from "../src/PoseidonHasher.sol";
import {Verifier} from "../src/veil-wallet/Verifier.sol";
import {AccountFactory} from "../src/veil-wallet/AccountFactory.sol";
import {VeilToken} from "../src/veil-wallet/VeilToken.sol";
import {IEntryPoint} from "@openzeppelin/contracts/interfaces/draft-IERC4337.sol";

/**
 * @title DeployVeilWalletContracts
 * @notice Deploy all VeilWallet contracts on Mantle
 * @dev Usage: forge script script/deployVeilWallet.s.sol:DeployVeilWalletContracts --rpc-url $MANTLE_RPC_URL --broadcast --verify
 */
contract DeployVeilWalletContracts is Script {
    // Canonical ERC-4337 EntryPoint address (same on all chains)
    address constant ENTRY_POINT = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        
        // Check deployer balance
        uint256 balance = deployer.balance;
        console.log("Deployer balance:", balance / 1e18, "MNT");
        console.log("Network:", block.chainid == 5000 ? "Mantle Mainnet" : block.chainid == 5003 ? "Mantle Sepolia Testnet" : "Unknown");
        console.log("Explorer:", block.chainid == 5000 ? "https://explorer.mantle.xyz" : block.chainid == 5003 ? "https://explorer.sepolia.mantle.xyz" : "N/A");
        require(balance > 0.5 ether, "Insufficient balance. Need at least 0.5 MNT for deployment");
        
        // Get PoseidonHasher address (can be existing or deploy new)
        address poseidonHasher = vm.envOr("POSEIDON_HASHER_ADDRESS", address(0));
        
        console.log("=== VeilWallet Contracts Deployment (Mantle) ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("EntryPoint:", ENTRY_POINT);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // Step 1: Deploy or use existing PoseidonHasher
        if (poseidonHasher == address(0)) {
            console.log("Deploying PoseidonHasher...");
            PoseidonHasher poseidon = new PoseidonHasher();
            poseidonHasher = address(poseidon);
            console.log("PoseidonHasher deployed at:", poseidonHasher);
            console.log("Gas used for PoseidonHasher:", gasleft());
        } else {
            console.log("Using existing PoseidonHasher at:", poseidonHasher);
        }
        console.log("");
        
        // Step 2: Deploy Verifier (smallest contract, ~200k gas)
        console.log("Deploying Verifier...");
        Verifier verifier = new Verifier(poseidonHasher);
        console.log("Verifier deployed at:", address(verifier));
        console.log("");
        
        // Step 3: Deploy AccountFactory (medium contract, ~1.2M gas)
        console.log("Deploying AccountFactory...");
        IEntryPoint entryPoint = IEntryPoint(ENTRY_POINT);
        AccountFactory factory = new AccountFactory(entryPoint);
        console.log("AccountFactory deployed at:", address(factory));
        console.log("");
        
        // Step 4: Deploy VeilToken (largest contract, ~1M gas)
        console.log("Deploying VeilToken...");
        VeilToken token = new VeilToken(
            "Veil Token",
            "VEIL",
            address(verifier)
        );
        console.log("VeilToken deployed at:", address(token));
        console.log("");
        
        vm.stopBroadcast();
        
        console.log("=== VEILWALLET DEPLOYMENT SUMMARY ===");
        console.log("PoseidonHasher:", poseidonHasher);
        console.log("Verifier:", address(verifier));
        console.log("AccountFactory:", address(factory));
        console.log("VeilToken:", address(token));
        console.log("");
        console.log("Add to .env:");
        console.log("POSEIDON_HASHER_ADDRESS=", poseidonHasher);
        console.log("VEIL_VERIFIER_ADDRESS=", address(verifier));
        console.log("VEIL_ACCOUNT_FACTORY_ADDRESS=", address(factory));
        console.log("VEIL_TOKEN_ADDRESS=", address(token));
        console.log("");
        console.log("EntryPoint (canonical):", ENTRY_POINT);
    }
}

/**
 * @title DeployVeilWalletContractsEthereum
 * @notice Deploy all VeilWallet contracts on Ethereum
 * @dev Usage: forge script script/deployVeilWallet.s.sol:DeployVeilWalletContractsEthereum --rpc-url $ETHEREUM_RPC_URL --broadcast --verify
 */
contract DeployVeilWalletContractsEthereum is Script {
    address constant ENTRY_POINT = 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789;
    
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        
        address poseidonHasher = vm.envOr("POSEIDON_HASHER_ADDRESS", address(0));
        
        console.log("=== VeilWallet Contracts Deployment (Ethereum) ===");
        console.log("Deployer:", deployer);
        console.log("Chain ID:", block.chainid);
        console.log("EntryPoint:", ENTRY_POINT);
        console.log("");
        
        vm.startBroadcast(deployerPrivateKey);
        
        if (poseidonHasher == address(0)) {
            console.log("Deploying PoseidonHasher...");
            PoseidonHasher poseidon = new PoseidonHasher();
            poseidonHasher = address(poseidon);
            console.log("PoseidonHasher deployed at:", poseidonHasher);
        } else {
            console.log("Using existing PoseidonHasher at:", poseidonHasher);
        }
        console.log("");
        
        console.log("Deploying Verifier...");
        Verifier verifier = new Verifier(poseidonHasher);
        console.log("Verifier deployed at:", address(verifier));
        console.log("");
        
        console.log("Deploying AccountFactory...");
        IEntryPoint entryPoint = IEntryPoint(ENTRY_POINT);
        AccountFactory factory = new AccountFactory(entryPoint);
        console.log("AccountFactory deployed at:", address(factory));
        console.log("");
        
        console.log("Deploying VeilToken...");
        VeilToken token = new VeilToken(
            "Veil Token",
            "VEIL",
            address(verifier)
        );
        console.log("VeilToken deployed at:", address(token));
        console.log("");
        
        vm.stopBroadcast();
        
        console.log("=== VEILWALLET DEPLOYMENT SUMMARY (ETHEREUM) ===");
        console.log("PoseidonHasher:", poseidonHasher);
        console.log("Verifier:", address(verifier));
        console.log("AccountFactory:", address(factory));
        console.log("VeilToken:", address(token));
        console.log("");
        console.log("Add to .env:");
        console.log("POSEIDON_HASHER_ADDRESS=", poseidonHasher);
        console.log("VEIL_VERIFIER_ADDRESS_ETH=", address(verifier));
        console.log("VEIL_ACCOUNT_FACTORY_ADDRESS_ETH=", address(factory));
        console.log("VEIL_TOKEN_ADDRESS_ETH=", address(token));
    }
}

