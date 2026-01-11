/**
 * Contract configuration and ABIs for Mantle-Ethereum Privacy Bridge
 *
 * Contains contract addresses and ABIs for PrivateIntentPool contracts
 * on Ethereum Sepolia and Mantle Sepolia testnets
 */

import type { Hex } from "viem";
import type { ChainType } from "./tokens";

/**
 * Contract addresses for Ethereum Sepolia Testnet (Chain ID: 11155111)
 */
export const ETHEREUM_CONTRACTS = {
  intentPool: "0xA019C4fFC50b58AF6221F9EB59126630E26f3326" as Hex,
  settlement: "0x0d632dC7786CD01712ED3CF3c6fa7F9e9C67Fa49" as Hex,
} as const;

/**
 * Contract addresses for Mantle Sepolia Testnet (Chain ID: 5003)
 */
export const MANTLE_CONTRACTS = {
  intentPool: "0x3ecB330Aa3EaE12bbc26022dE9ee140C06e3da43" as Hex,
  settlement: "0x67eEf23fc45e149E43944100be8658c55D48f1a1" as Hex,
  poseidonHasher: "0x8EA86eD4317AF92f73E5700eB9b93A72dE62f3B1" as Hex,
} as const;

/**
 * Get contract addresses for a specific chain
 */
export function getContractsForChain(chain: ChainType) {
  return chain === "ethereum" ? ETHEREUM_CONTRACTS : MANTLE_CONTRACTS;
}

/**
 * PrivateIntentPool Contract ABI
 * Only includes functions needed for frontend integration
 */
export const INTENT_POOL_ABI = [
  {
    type: "function",
    name: "createIntent",
    stateMutability: "payable",
    inputs: [
      {
        name: "intentId",
        type: "bytes32",
        internalType: "bytes32",
      },
      {
        name: "commitment",
        type: "bytes32",
        internalType: "bytes32",
      },
      {
        name: "token",
        type: "address",
        internalType: "address",
      },
      {
        name: "amount",
        type: "uint256",
        internalType: "uint256",
      },
      {
        name: "destChain",
        type: "uint32",
        internalType: "uint32",
      },
      {
        name: "refundTo",
        type: "address",
        internalType: "address",
      },
      {
        name: "customDeadline",
        type: "uint64",
        internalType: "uint64",
      },
    ],
    outputs: [],
  },
  {
    type: "event",
    name: "IntentCreated",
    inputs: [
      {
        name: "intentId",
        type: "bytes32",
        indexed: true,
        internalType: "bytes32",
      },
      {
        name: "commitment",
        type: "bytes32",
        indexed: false,
        internalType: "bytes32",
      },
      {
        name: "creator",
        type: "address",
        indexed: true,
        internalType: "address",
      },
      {
        name: "token",
        type: "address",
        indexed: true,
        internalType: "address",
      },
      {
        name: "amount",
        type: "uint256",
        indexed: false,
        internalType: "uint256",
      },
      {
        name: "destChain",
        type: "uint32",
        indexed: false,
        internalType: "uint32",
      },
      {
        name: "deadline",
        type: "uint256",
        indexed: false,
        internalType: "uint256",
      },
    ],
  },
] as const;

/**
 * ERC20 Token ABI
 * Only includes approve function needed for bridge
 */
export const ERC20_ABI = [
  {
    type: "function",
    name: "approve",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "spender",
        type: "address",
        internalType: "address",
      },
      {
        name: "amount",
        type: "uint256",
        internalType: "uint256",
      },
    ],
    outputs: [
      {
        name: "",
        type: "bool",
        internalType: "bool",
      },
    ],
  },
  {
    type: "function",
    name: "allowance",
    stateMutability: "view",
    inputs: [
      {
        name: "owner",
        type: "address",
        internalType: "address",
      },
      {
        name: "spender",
        type: "address",
        internalType: "address",
      },
    ],
    outputs: [
      {
        name: "",
        type: "uint256",
        internalType: "uint256",
      },
    ],
  },
  {
    type: "function",
    name: "balanceOf",
    stateMutability: "view",
    inputs: [
      {
        name: "account",
        type: "address",
        internalType: "address",
      },
    ],
    outputs: [
      {
        name: "",
        type: "uint256",
        internalType: "uint256",
      },
    ],
  },
  {
    type: "function",
    name: "decimals",
    stateMutability: "view",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "uint8",
        internalType: "uint8",
      },
    ],
  },
] as const;

/**
 * Chain ID mapping
 */
export const CHAIN_IDS = {
  ethereum: 11155111, // Ethereum Sepolia
  mantle: 5003, // Mantle Sepolia
} as const;

/**
 * Get chain ID for a chain type
 */
export function getChainId(chain: ChainType): number {
  return CHAIN_IDS[chain];
}

/**
 * Get chain type from chain ID
 */
export function getChainType(chainId: number): ChainType | undefined {
  if (chainId === CHAIN_IDS.ethereum) return "ethereum";
  if (chainId === CHAIN_IDS.mantle) return "mantle";
  return undefined;
}

/**
 * Block explorer URLs
 */
export const BLOCK_EXPLORERS = {
  ethereum: "https://sepolia.etherscan.io",
  mantle: "https://explorer.sepolia.mantle.xyz",
} as const;

/**
 * Get block explorer URL for a chain
 */
export function getExplorerUrl(chain: ChainType): string {
  return BLOCK_EXPLORERS[chain];
}

/**
 * Get transaction URL on block explorer
 */
export function getTxUrl(chain: ChainType, txHash: Hex): string {
  return `${getExplorerUrl(chain)}/tx/${txHash}`;
}

/**
 * Get address URL on block explorer
 */
export function getAddressUrl(chain: ChainType, address: Hex): string {
  return `${getExplorerUrl(chain)}/address/${address}`;
}

/**
 * Validate contract addresses are configured
 */
export function validateContractAddresses(): void {
  if (!ETHEREUM_CONTRACTS.intentPool) {
    throw new Error("Ethereum IntentPool address not configured");
  }
  if (!MANTLE_CONTRACTS.intentPool) {
    throw new Error("Mantle IntentPool address not configured");
  }
}
