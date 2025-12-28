/**
 * Token configuration for Mantle-Ethereum Privacy Bridge
 *
 * Defines supported tokens on Ethereum Sepolia and Mantle Sepolia testnets
 */

import type { Hex } from "viem";

/**
 * Supported chains
 */
export type ChainType = "ethereum" | "mantle";

/**
 * Token metadata
 */
export interface TokenInfo {
  symbol: string;
  name: string;
  decimals: number;
  address: Hex;
  isNative: boolean;
  logo?: string;
}

/**
 * Token addresses on Ethereum Sepolia Testnet (Chain ID: 11155111)
 */
export const ETHEREUM_TOKENS: Record<string, TokenInfo> = {
  ETH: {
    symbol: "ETH",
    name: "Ethereum",
    decimals: 18,
    address: "0x0000000000000000000000000000000000000000",
    isNative: true,
  },
  USDC: {
    symbol: "USDC",
    name: "USD Coin",
    decimals: 6,
    address: "0x28650373758d75a8fF0B22587F111e47BAC34e21",
    isNative: false,
  },
  USDT: {
    symbol: "USDT",
    name: "Tether USD",
    decimals: 6,
    address: "0x89F4f0e13997Ca27cEB963DEE291C607e4E59923",
    isNative: false,
  },
  WETH: {
    symbol: "WETH",
    name: "Wrapped Ether",
    decimals: 18,
    address: "0x50e8Da97BeEB8064714dE45ce1F250879f3bD5B5",
    isNative: false,
  },
  MNT: {
    symbol: "MNT",
    name: "Mantle Token",
    decimals: 18,
    address: "0x65e37B558F64e2Be5768DB46DF22F93d85741A9E",
    isNative: false,
  },
};

/**
 * Token addresses on Mantle Sepolia Testnet (Chain ID: 5003)
 */
export const MANTLE_TOKENS: Record<string, TokenInfo> = {
  ETH: {
    symbol: "ETH",
    name: "Ethereum",
    decimals: 18,
    address: "0x0000000000000000000000000000000000000000",
    isNative: true,
  },
  USDC: {
    symbol: "USDC",
    name: "USD Coin",
    decimals: 6,
    address: "0xA4b184006B59861f80521649b14E4E8A72499A23",
    isNative: false,
  },
  USDT: {
    symbol: "USDT",
    name: "Tether USD",
    decimals: 6,
    address: "0xB0ee6EF7788E9122fc4AAE327Ed4FEf56c7da891",
    isNative: false,
  },
  WETH: {
    symbol: "WETH",
    name: "Wrapped Ether",
    decimals: 18,
    address: "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111",
    isNative: false,
  },
  MNT: {
    symbol: "MNT",
    name: "Mantle Token",
    decimals: 18,
    address: "0x44FCE297e4D6c5A50D28Fb26A58202e4D49a13E7",
    isNative: false,
  },
};

/**
 * Get tokens for a specific chain
 */
export function getTokensForChain(chain: ChainType): Record<string, TokenInfo> {
  return chain === "ethereum" ? ETHEREUM_TOKENS : MANTLE_TOKENS;
}

/**
 * Get token info by symbol and chain
 */
export function getTokenInfo(
  symbol: string,
  chain: ChainType
): TokenInfo | undefined {
  const tokens = getTokensForChain(chain);
  return tokens[symbol.toUpperCase()];
}

/**
 * Get token address by symbol and chain
 */
export function getTokenAddress(symbol: string, chain: ChainType): Hex {
  const token = getTokenInfo(symbol, chain);
  if (!token) {
    throw new Error(`Token ${symbol} not found on ${chain}`);
  }
  return token.address;
}

/**
 * Check if token is native (ETH/MNT)
 */
export function isNativeToken(address: Hex): boolean {
  return (
    address === "0x0000000000000000000000000000000000000000" ||
    address.toLowerCase() ===
      "0x0000000000000000000000000000000000000000".toLowerCase()
  );
}

/**
 * Get list of supported token symbols
 */
export const SUPPORTED_TOKENS = ["ETH", "USDC", "USDT", "WETH", "MNT"] as const;

export type SupportedToken = (typeof SUPPORTED_TOKENS)[number];

/**
 * Validate if token is supported on a chain
 */
export function isSupportedToken(
  symbol: string,
  chain: ChainType
): symbol is SupportedToken {
  const tokens = getTokensForChain(chain);
  return symbol.toUpperCase() in tokens;
}

/**
 * Format token amount for display
 */
export function formatTokenAmount(
  amount: bigint,
  decimals: number,
  maxDecimals: number = 6
): string {
  const divisor = BigInt(10 ** decimals);
  const quotient = amount / divisor;
  const remainder = amount % divisor;

  if (remainder === BigInt(0)) {
    return quotient.toString();
  }

  const remainderStr = remainder.toString().padStart(decimals, "0");
  const trimmedRemainder = remainderStr.slice(0, maxDecimals).replace(/0+$/, "");

  if (trimmedRemainder === "") {
    return quotient.toString();
  }

  return `${quotient}.${trimmedRemainder}`;
}

/**
 * Parse token amount from string to bigint
 */
export function parseTokenAmount(amount: string, decimals: number): bigint {
  const [whole = "0", fraction = "0"] = amount.split(".");

  // Pad or trim fraction to match decimals
  const paddedFraction = fraction.padEnd(decimals, "0").slice(0, decimals);

  const amountStr = whole + paddedFraction;
  return BigInt(amountStr);
}
