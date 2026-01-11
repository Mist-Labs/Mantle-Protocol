/**
 * ECIES Encryption Utilities for Privacy Parameters
 *
 * This module handles encryption of sensitive privacy parameters (secret and nullifier)
 * using ECIES (Elliptic Curve Integrated Encryption Scheme) before transmission to the backend.
 *
 * Security Model:
 * - Secret and nullifier are encrypted client-side with the relayer's public key
 * - Only the relayer can decrypt using their private key
 * - Prevents exposure of sensitive values during API transmission
 */

import { encrypt } from "eciesjs";
import type { Hex } from "viem";

/**
 * Get relayer's public key from environment
 */
const RELAYER_PUBLIC_KEY = process.env.NEXT_PUBLIC_RELAYER_PUBLIC_KEY;

/**
 * Validate relayer public key is configured
 */
function validateRelayerPublicKey(): void {
  if (!RELAYER_PUBLIC_KEY) {
    throw new Error(
      "Relayer public key not configured. Please set NEXT_PUBLIC_RELAYER_PUBLIC_KEY environment variable."
    );
  }
}

/**
 * Encrypt data using ECIES with the relayer's public key
 *
 * @param data - Hex string to encrypt (with or without 0x prefix)
 * @returns Promise<string> - Encrypted data as 0x-prefixed hex string
 *
 * @example
 * const secret = "0x1234...abcd";
 * const encryptedSecret = await encryptForRelayer(secret);
 * // Use encryptedSecret in API call
 */
export async function encryptForRelayer(data: Hex | string): Promise<string> {
  validateRelayerPublicKey();

  // Remove 0x prefix if present
  const cleanData = data.startsWith("0x") ? data.slice(2) : data;
  const cleanPubKey = RELAYER_PUBLIC_KEY!.startsWith("0x")
    ? RELAYER_PUBLIC_KEY!.slice(2)
    : RELAYER_PUBLIC_KEY!;

  // Convert hex strings to Buffers
  const publicKeyBuffer = Buffer.from(cleanPubKey, "hex");
  const dataBuffer = Buffer.from(cleanData, "hex");

  // Encrypt using ECIES
  const encrypted = encrypt(publicKeyBuffer, dataBuffer);

  // Return as 0x-prefixed hex string
  return "0x" + encrypted.toString("hex");
}

/**
 * Encrypt privacy parameters (secret and nullifier) for API transmission
 *
 * @param secret - 32-byte secret hex string
 * @param nullifier - 32-byte nullifier hex string
 * @returns Promise with encrypted values
 *
 * @example
 * const { encryptedSecret, encryptedNullifier } = await encryptPrivacyParams(
 *   "0x1234...secret",
 *   "0x5678...nullifier"
 * );
 */
export async function encryptPrivacyParams(
  secret: Hex,
  nullifier: Hex
): Promise<{
  encryptedSecret: string;
  encryptedNullifier: string;
}> {
  // Encrypt both parameters concurrently
  const [encryptedSecret, encryptedNullifier] = await Promise.all([
    encryptForRelayer(secret),
    encryptForRelayer(nullifier),
  ]);

  return {
    encryptedSecret,
    encryptedNullifier,
  };
}

/**
 * Validate that encryption is properly configured
 * Call this during app initialization to catch configuration errors early
 */
export function validateEncryptionConfig(): void {
  validateRelayerPublicKey();
}
