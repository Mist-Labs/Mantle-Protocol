/**
 * Privacy utilities for the Mantle-Ethereum Privacy Bridge
 *
 * This module handles client-side generation of privacy parameters:
 * - Secret: Random 32-byte value (NEVER goes on-chain)
 * - Nullifier: Derived from secret (prevents double-spend)
 * - Commitment: Hash of intentId + secret + nullifier (goes on-chain)
 * - IntentId: Unique identifier for each bridge intent
 *
 * Security Model:
 * - Secret is generated client-side and only shared with backend after on-chain confirmation
 * - Commitment proves knowledge of secret without revealing it
 * - Nullifier ensures secret can only be used once
 */

import {
  encodeAbiParameters,
  keccak256,
  type Hex,
  parseAbiParameters,
} from "viem";
import { generatePrivateKey } from "viem/accounts";

/**
 * Generate a cryptographically secure random 32-byte secret
 * @returns 32-byte hex string (66 chars with 0x prefix)
 */
export function generateSecret(): Hex {
  // Use viem's generatePrivateKey which creates a secure random 32-byte value
  return generatePrivateKey();
}

/**
 * Derive nullifier from secret using keccak256 hash
 * This prevents double-spending by making each secret usable only once
 *
 * @param secret - 32-byte hex string
 * @returns 32-byte hex string (66 chars with 0x prefix)
 */
export function generateNullifier(secret: Hex): Hex {
  const encoded = encodeAbiParameters(
    parseAbiParameters("bytes32"),
    [secret]
  );
  return keccak256(encoded);
}

/**
 * Generate a unique intent ID for the bridge transaction
 * Uses user address, token, amount, timestamp, and random nonce for uniqueness
 *
 * @param userAddress - User's wallet address
 * @param token - Token contract address (0x0 for native)
 * @param amount - Amount in wei (bigint)
 * @returns 32-byte hex string (66 chars with 0x prefix)
 */
export function generateIntentId(
  userAddress: Hex,
  token: Hex,
  amount: bigint
): Hex {
  const timestamp = BigInt(Math.floor(Date.now() / 1000));
  const nonce = BigInt(Math.floor(Math.random() * 1000000));

  const encoded = encodeAbiParameters(
    parseAbiParameters("address, address, uint256, uint32, uint256"),
    [userAddress, token, amount, Number(timestamp), nonce]
  );

  return keccak256(encoded);
}

/**
 * Compute commitment hash from intentId, secret, and nullifier
 * This is what gets stored on-chain - it proves you know the secret without revealing it
 *
 * @param intentId - Unique intent identifier
 * @param secret - 32-byte secret
 * @param nullifier - 32-byte nullifier (derived from secret)
 * @returns 32-byte hex string (66 chars with 0x prefix)
 */
export function generateCommitment(
  intentId: Hex,
  secret: Hex,
  nullifier: Hex
): Hex {
  const encoded = encodeAbiParameters(
    parseAbiParameters("bytes32, bytes32, bytes32"),
    [intentId, secret, nullifier]
  );
  return keccak256(encoded);
}

/**
 * Generate complete privacy parameters for a bridge transaction
 * This is the main function to call when initiating a bridge
 *
 * @param userAddress - User's wallet address
 * @param token - Token contract address (0x0 for native)
 * @param amount - Amount in wei (bigint)
 * @returns Object containing all privacy parameters
 */
export function generatePrivacyParams(
  userAddress: Hex,
  token: Hex,
  amount: bigint
) {
  // Step 1: Generate random secret (client-side only)
  const secret = generateSecret();

  // Step 2: Derive nullifier from secret
  const nullifier = generateNullifier(secret);

  // Step 3: Generate unique intent ID
  const intentId = generateIntentId(userAddress, token, amount);

  // Step 4: Compute commitment (this goes on-chain)
  const commitment = generateCommitment(intentId, secret, nullifier);

  return {
    intentId,
    secret,
    nullifier,
    commitment,
  };
}

/**
 * Validate privacy parameter formats
 * Ensures all parameters are properly formatted 32-byte hex strings
 *
 * @param params - Privacy parameters to validate
 * @returns true if valid, throws error otherwise
 */
export function validatePrivacyParams(params: {
  intentId: Hex;
  secret: Hex;
  nullifier: Hex;
  commitment: Hex;
}): boolean {
  const validate32ByteHex = (value: string, name: string) => {
    if (!value.startsWith("0x")) {
      throw new Error(`${name} must start with 0x`);
    }
    if (value.length !== 66) {
      throw new Error(
        `${name} must be 66 characters (32 bytes with 0x prefix)`
      );
    }
    if (!/^0x[0-9a-fA-F]{64}$/.test(value)) {
      throw new Error(`${name} must be a valid hex string`);
    }
  };

  validate32ByteHex(params.intentId, "intentId");
  validate32ByteHex(params.secret, "secret");
  validate32ByteHex(params.nullifier, "nullifier");
  validate32ByteHex(params.commitment, "commitment");

  return true;
}

/**
 * EIP-712 Domain for claim authorization signatures
 * This defines the signing domain for the destination chain settlement contract
 *
 * @param chainId - Destination chain ID (11155111 for Sepolia, 5003 for Mantle Sepolia)
 * @param contractAddress - Settlement contract address on destination chain
 * @returns EIP-712 domain object
 */
export function getEIP712Domain(chainId: number, contractAddress: Hex) {
  return {
    name: "IntentSettlement",
    version: "1",
    chainId,
    verifyingContract: contractAddress,
  } as const;
}

/**
 * EIP-712 types for claim authorization
 * Defines the structure of the ClaimAuthorization message
 */
export const CLAIM_AUTH_TYPES = [
  { name: "intentId", type: "bytes32" },
  { name: "recipient", type: "address" },
  { name: "token", type: "address" },
  { name: "amount", type: "uint256" },
] as const;

/**
 * Type definition for claim authorization message
 */
export type ClaimAuthMessage = {
  intentId: Hex;
  recipient: Hex;
  token: Hex;
  amount: bigint;
};

/**
 * Validate claim authorization signature format
 *
 * @param signature - Signature to validate
 * @returns true if valid, throws error otherwise
 */
export function validateClaimAuth(signature: string): boolean {
  if (!signature.startsWith("0x")) {
    throw new Error("Signature must start with 0x");
  }
  if (signature.length !== 132) {
    throw new Error(
      "Signature must be 132 characters (65 bytes with 0x prefix)"
    );
  }
  if (!/^0x[0-9a-fA-F]{130}$/.test(signature)) {
    throw new Error("Signature must be a valid hex string");
  }
  return true;
}
