/**
 * API Client for Mantle-Ethereum Privacy Bridge Backend
 *
 * Handles all communication with the backend relayer API including:
 * - HMAC authentication
 * - Bridge intent initiation
 * - Intent status polling
 * - Price feed queries
 * - Automatic retry on network failures
 */

import type { ChainType } from "./tokens";
import { retryAPICall } from "./retry";
import {
  APIError,
  HMACAuthError,
  IntentNotFoundError,
  NetworkTimeoutError,
} from "./errors";

/**
 * API Configuration
 */
const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ||
  "https://international-linnie-mist-labs-2c5cd590.koyeb.app/api/v1";

const HMAC_SECRET = process.env.NEXT_PUBLIC_HMAC_SECRET || "";

/**
 * Intent status types
 */
export type IntentStatus =
  | "created"
  | "committed"
  | "filled"
  | "completed"
  | "refunded"
  | "failed";

/**
 * Bridge intent initiation request
 * Updated to match API v1.0.0 specification
 */
export interface BridgeInitiateRequest {
  intent_id: string;
  user_address: string;
  source_chain: ChainType;
  dest_chain: ChainType;
  source_token: string;
  dest_token: string;
  amount: string;
  commitment: string;
  encrypted_secret: string; // ECIES encrypted secret
  encrypted_nullifier: string; // ECIES encrypted nullifier
  claim_auth: string;
  recipient: string;
  refund_address: string;
}

/**
 * Bridge intent initiation response
 */
export interface BridgeInitiateResponse {
  success: boolean;
  intent_id: string;
  commitment: string;
  message: string;
  error: string | null;
}

/**
 * Backend raw response format (what API actually returns)
 */
interface BackendIntentResponse {
  id: string; // Backend uses "id" not "intent_id"
  status: string; // "Committed", "Filled", etc. (capitalized)
  source_chain: string;
  dest_chain: string;
  source_token: string;
  dest_token: string;
  amount: string;
  source_commitment: string; // Backend uses "source_commitment"
  dest_fill_txid: string | null;
  source_complete_txid: string | null;
  dest_registration_txid?: string | null;
  deadline: number;
  created_at: string;
  updated_at: string;
  user_address: string;
  refund_address: string;
  solver_address?: string | null;
  block_number?: number;
  log_index?: number;
}

/**
 * Normalized intent status response (what frontend uses)
 */
export interface IntentStatusResponse {
  intent_id: string;
  status: IntentStatus;
  source_chain: string;
  dest_chain: string;
  source_token: string;
  dest_token: string;
  amount: string;
  commitment: string;
  dest_fill_txid: string | null;
  source_complete_txid: string | null;
  deadline: number;
  created_at: string;
  updated_at: string;
  has_privacy: boolean;
  user_address?: string; // For client-side filtering
}

/**
 * Normalize backend response to frontend format
 * Handles field name differences between backend and frontend
 */
function normalizeIntentResponse(
  backend: BackendIntentResponse
): IntentStatusResponse {
  // Map backend status names to frontend status names
  const statusMap: Record<string, IntentStatus> = {
    Committed: "committed",
    Registered: "committed",
    Filled: "filled",
    SolverPaid: "completed",
    UserClaimed: "completed",
    Refunded: "refunded",
    Failed: "failed",
  };

  return {
    intent_id: backend.id, // Map "id" to "intent_id"
    status: statusMap[backend.status] || "created",
    source_chain: backend.source_chain,
    dest_chain: backend.dest_chain,
    source_token: backend.source_token,
    dest_token: backend.dest_token,
    amount: backend.amount,
    commitment: backend.source_commitment, // Map "source_commitment" to "commitment"
    dest_fill_txid: backend.dest_fill_txid,
    source_complete_txid: backend.source_complete_txid,
    deadline: backend.deadline,
    created_at: backend.created_at,
    updated_at: backend.updated_at,
    has_privacy: true,
    user_address: backend.user_address, // Preserve for client-side filtering
  };
}

/**
 * Price query response
 */
export interface PriceResponse {
  from_symbol: string;
  to_symbol: string;
  rate: number;
  amount: number;
  converted_amount: number;
  timestamp: number;
  sources: Array<{
    source: string;
    price: number;
  }>;
}

/**
 * Generate HMAC signature for API requests
 * Uses Web Crypto API (browser-compatible)
 *
 * @param payload - Request body object
 * @param timestamp - Unix timestamp in seconds
 * @returns Promise<string> - Hex signature
 */
async function generateHMACSignature(
  payload: Record<string, unknown>,
  timestamp: string
): Promise<string> {
  if (!HMAC_SECRET) {
    throw new Error(
      "HMAC secret not configured. Please set NEXT_PUBLIC_HMAC_SECRET environment variable."
    );
  }

  const requestBody = JSON.stringify(payload);
  const message = timestamp + requestBody;

  // Convert secret and message to Uint8Array
  const encoder = new TextEncoder();
  const keyData = encoder.encode(HMAC_SECRET);
  const messageData = encoder.encode(message);

  // Import key for HMAC
  const key = await crypto.subtle.importKey(
    "raw",
    keyData,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );

  // Sign the message
  const signature = await crypto.subtle.sign("HMAC", key, messageData);

  // Convert to hex string
  const hashArray = Array.from(new Uint8Array(signature));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

  return hashHex;
}

/**
 * Make authenticated API request with HMAC signature
 *
 * @param endpoint - API endpoint path
 * @param options - Fetch options
 * @returns Promise<Response>
 */
async function authenticatedFetch(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `${API_BASE_URL}${endpoint}`;
  const timestamp = Math.floor(Date.now() / 1000).toString();

  let body: Record<string, unknown> = {};
  if (options.body) {
    body = JSON.parse(options.body as string);
  }

  const signature = await generateHMACSignature(body, timestamp);

  const headers: HeadersInit = {
    "Content-Type": "application/json",
    "X-Signature": signature,
    "X-Timestamp": timestamp,
    ...((options.headers as Record<string, string>) || {}),
  };

  return fetch(url, {
    ...options,
    headers,
  });
}

/**
 * Initiate a bridge transaction
 *
 * @param request - Bridge initiation request
 * @returns Promise<BridgeInitiateResponse>
 */
export async function initiateBridge(
  request: BridgeInitiateRequest
): Promise<BridgeInitiateResponse> {
  return retryAPICall(async () => {
    try {
      const response = await authenticatedFetch("/bridge/initiate", {
        method: "POST",
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        if (response.status === 401) {
          throw new HMACAuthError();
        }

        const error = await response.json().catch(() => ({
          message: "Failed to initiate bridge",
        }));
        throw new APIError(
          response.status,
          error.message || error.error || "Failed to initiate bridge"
        );
      }

      return response.json();
    } catch (error) {
      if (error instanceof HMACAuthError || error instanceof APIError) {
        throw error;
      }
      throw new NetworkTimeoutError(error instanceof Error ? error : undefined);
    }
  }, 3);
}

/**
 * Get intent status by ID
 *
 * @param intentId - Intent ID
 * @returns Promise<IntentStatusResponse>
 */
export async function getIntentStatus(
  intentId: string
): Promise<IntentStatusResponse> {
  return retryAPICall(async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/bridge/intent/${intentId}`);

      if (!response.ok) {
        if (response.status === 404) {
          throw new IntentNotFoundError(intentId);
        }
        throw new APIError(response.status, "Failed to fetch intent status");
      }

      const backendData: BackendIntentResponse = await response.json();
      return normalizeIntentResponse(backendData);
    } catch (error) {
      if (error instanceof IntentNotFoundError || error instanceof APIError) {
        throw error;
      }
      throw new NetworkTimeoutError(error instanceof Error ? error : undefined);
    }
  }, 3);
}

/**
 * Poll intent status until it reaches a terminal state
 *
 * @param intentId - Intent ID to poll
 * @param onUpdate - Callback for status updates
 * @param maxAttempts - Maximum polling attempts (default: 60)
 * @param intervalMs - Polling interval in ms (default: 5000)
 * @returns Promise<IntentStatusResponse> - Returns last known status even if terminal state not reached
 */
export async function pollIntentStatus(
  intentId: string,
  onUpdate?: (status: IntentStatusResponse) => void,
  maxAttempts: number = 60,
  intervalMs: number = 5000
): Promise<IntentStatusResponse> {
  const terminalStates: IntentStatus[] = ["completed", "refunded", "failed"];
  let lastStatus: IntentStatusResponse | null = null;

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    const status = await getIntentStatus(intentId);
    lastStatus = status;

    // Call update callback
    if (onUpdate) {
      onUpdate(status);
    }

    // Check if terminal state reached
    if (terminalStates.includes(status.status)) {
      return status;
    }

    // Wait before next poll
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  // If we reach here, polling timed out but transaction is still valid
  // Return the last known status instead of throwing an error
  console.log("✅ Polling completed without reaching terminal state. Transaction is still processing.");
  return lastStatus!;
}

/**
 * Get exchange rate between two tokens
 *
 * @param fromSymbol - Source token symbol
 * @param toSymbol - Destination token symbol
 * @param amount - Amount to convert (optional)
 * @returns Promise<PriceResponse>
 */
export async function getExchangeRate(
  fromSymbol: string,
  toSymbol: string,
  amount?: number
): Promise<PriceResponse> {
  const params = new URLSearchParams({
    from_symbol: fromSymbol,
    to_symbol: toSymbol,
  });

  if (amount !== undefined) {
    params.append("amount", amount.toString());
  }

  const response = await fetch(`${API_BASE_URL}/price?${params}`);

  if (!response.ok) {
    throw new Error("Failed to fetch exchange rate");
  }

  return response.json();
}

/**
 * Health check endpoint
 *
 * @returns Promise<{ status: string; components: Record<string, string> }>
 */
export async function healthCheck(): Promise<{
  status: string;
  timestamp: string;
  components: Record<string, string>;
}> {
  const response = await fetch(`${API_BASE_URL}/health`);

  if (!response.ok) {
    throw new Error("Health check failed");
  }

  return response.json();
}

/**
 * Get bridge statistics
 *
 * @returns Promise<Record<string, unknown>>
 */
export async function getBridgeStats(): Promise<{
  status: string;
  data: {
    total_intents: number;
    pending_intents: number;
    filled_intents: number;
    completed_intents: number;
    failed_intents: number;
    refunded_intents: number;
    ethereum_to_mantle: number;
    mantle_to_ethereum: number;
    total_volume_by_token: Record<string, string>;
  };
}> {
  const response = await fetch(`${API_BASE_URL}/stats`);

  if (!response.ok) {
    throw new Error("Failed to fetch bridge stats");
  }

  return response.json();
}

/**
 * List bridge intents with optional filters
 *
 * @param options - Filter options
 * @returns Promise<IntentStatusResponse[]>
 */
export async function listBridgeIntents(options?: {
  status?: IntentStatus;
  chain?: ChainType;
  limit?: number;
  userAddress?: string;
}): Promise<{
  status: string;
  count: number;
  data: IntentStatusResponse[];
}> {
  const params = new URLSearchParams();

  if (options?.status) params.append("status", options.status);
  if (options?.chain) params.append("chain", options.chain);
  if (options?.limit) params.append("limit", options.limit.toString());
  if (options?.userAddress) params.append("user_address", options.userAddress);

  const queryString = params.toString();
  const url = `${API_BASE_URL}/bridge/intents${queryString ? `?${queryString}` : ""}`;

  const response = await fetch(url);

  if (!response.ok) {
    throw new Error("Failed to fetch bridge intents");
  }

  const backendResponse: {
    status: string;
    count: number;
    data: BackendIntentResponse[];
  } = await response.json();

  // Normalize all intent responses
  return {
    status: backendResponse.status,
    count: backendResponse.count,
    data: backendResponse.data.map(normalizeIntentResponse),
  };
}

/**
 * Get all token prices in USD
 *
 * @returns Promise<PricesAllResponse>
 */
export async function getAllPrices(): Promise<{
  status: string;
  timestamp: number;
  prices: Record<string, number>;
}> {
  const response = await fetch(`${API_BASE_URL}/prices/all`);

  if (!response.ok) {
    throw new Error("Failed to fetch all prices");
  }

  return response.json();
}

/**
 * Convert amount between tokens
 *
 * @param fromSymbol - Source token symbol
 * @param toSymbol - Destination token symbol
 * @param amount - Amount to convert
 * @returns Promise<ConvertResponse>
 */
export async function convertAmount(
  fromSymbol: string,
  toSymbol: string,
  amount: number
): Promise<{
  from_symbol: string;
  to_symbol: string;
  input_amount: number;
  output_amount: number;
  rate: number;
  timestamp: number;
}> {
  const response = await fetch(`${API_BASE_URL}/price/convert`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from_symbol: fromSymbol,
      to_symbol: toSymbol,
      amount,
    }),
  });

  if (!response.ok) {
    throw new Error("Failed to convert amount");
  }

  return response.json();
}

/**
 * Get API service information
 *
 * @returns Promise<ServiceInfo>
 */
export async function getServiceInfo(): Promise<{
  service: string;
  version: string;
  status: string;
  supported_chains: string[];
  supported_tokens: string[];
}> {
  const response = await fetch(`${API_BASE_URL}/`);

  if (!response.ok) {
    throw new Error("Failed to fetch service info");
  }

  return response.json();
}

/**
 * Validate API configuration
 */
export function validateAPIConfig(): void {
  if (!API_BASE_URL) {
    throw new Error("API base URL not configured");
  }
  if (!HMAC_SECRET) {
    console.warn(
      "HMAC secret not configured. Bridge transactions will fail. Please set NEXT_PUBLIC_HMAC_SECRET."
    );
  }
}
