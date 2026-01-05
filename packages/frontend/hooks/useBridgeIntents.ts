/**
 * useBridgeIntents Hook
 *
 * Hook for fetching and managing bridge intent transaction history
 */

import { useState, useEffect, useCallback } from "react";
import { listBridgeIntents, type IntentStatusResponse, type IntentStatus } from "@/lib/api";
import type { ChainType } from "@/lib/tokens";

/**
 * Bridge intents state
 */
interface BridgeIntentsState {
  intents: IntentStatusResponse[];
  count: number;
  isLoading: boolean;
  error: string | null;
}

/**
 * Filter options for bridge intents
 */
interface BridgeIntentsFilters {
  status?: IntentStatus;
  chain?: ChainType;
  limit?: number;
}

/**
 * Hook for fetching bridge intents with filtering
 */
export function useBridgeIntents(filters?: BridgeIntentsFilters) {
  const [state, setState] = useState<BridgeIntentsState>({
    intents: [],
    count: 0,
    isLoading: true,
    error: null,
  });

  const fetchIntents = useCallback(async () => {
    try {
      setState((prev) => ({ ...prev, isLoading: true, error: null }));

      const response = await listBridgeIntents(filters);

      setState({
        intents: response.data,
        count: response.count,
        isLoading: false,
        error: null,
      });
    } catch (error) {
      console.error("Failed to fetch bridge intents:", error);
      setState({
        intents: [],
        count: 0,
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to fetch intents",
      });
    }
  }, [filters?.status, filters?.chain, filters?.limit]);

  // Initial fetch
  useEffect(() => {
    fetchIntents();
  }, [fetchIntents]);

  return {
    ...state,
    refetch: fetchIntents,
  };
}

/**
 * Format time ago string
 */
export function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} minutes ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} hours ago`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)} days ago`;

  return date.toLocaleDateString();
}

/**
 * Format chain name for display
 */
export function formatChainName(chain: string): string {
  switch (chain.toLowerCase()) {
    case "ethereum":
      return "Ethereum Sepolia";
    case "mantle":
      return "Mantle Sepolia";
    default:
      return chain;
  }
}

/**
 * Format token amount for display
 */
export function formatAmount(amount: string, decimals: number = 18): string {
  const value = BigInt(amount);
  const divisor = BigInt(10 ** decimals);
  const quotient = value / divisor;
  const remainder = value % divisor;

  if (remainder === BigInt(0)) {
    return quotient.toString();
  }

  const remainderStr = remainder.toString().padStart(decimals, "0");
  const trimmedRemainder = remainderStr.slice(0, 6).replace(/0+$/, "");

  if (trimmedRemainder === "") {
    return quotient.toString();
  }

  return `${quotient}.${trimmedRemainder}`;
}
