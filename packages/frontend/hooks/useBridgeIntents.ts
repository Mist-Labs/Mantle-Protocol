/**
 * useBridgeIntents Hook
 *
 * Hook for fetching and managing bridge intent transaction history with real-time updates
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { listBridgeIntents, type IntentStatusResponse, type IntentStatus } from "@/lib/api";
import type { ChainType } from "@/lib/tokens";
import { useIntentPoolWatch } from "./useIntentPoolWatch";

/**
 * Filter options for bridge intents
 */
interface BridgeIntentsFilters {
  status?: IntentStatus;
  chain?: ChainType;
  limit?: number;
  userAddress?: string;
}

/**
 * Hook for fetching bridge intents with filtering and real-time updates
 */
export function useBridgeIntents(filters?: BridgeIntentsFilters) {
  // Watch Intent Pool contract events for real-time updates
  useIntentPoolWatch();

  // Query for backend intents (no local storage, backend only)
  const {
    data: backendData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    // Fetch ALL intents by removing status/chain filters from query key
    // This bypasses the backend API casing mismatch issue
    queryKey: ["bridgeIntents", "all", "all", filters?.limit, filters?.userAddress],
    queryFn: async () => {
      // Pass minimal filters to backend to get everything, then filter client-side
      return await listBridgeIntents({
        // Only pass limit and userAddress if needed
        limit: filters?.limit,
        userAddress: filters?.userAddress,
        // Explicitly undefined status/chain to fetch all
        status: undefined,
        chain: undefined
      });
    },
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) return 10000; // Poll every 10 seconds if no data

      // Check if there are pending transactions
      const hasPending = data.data.some((intent) => {
        const terminalStates = ["completed", "refunded", "failed"];
        return !terminalStates.includes(intent.status);
      });

      // Poll every 5 seconds if pending transactions exist, otherwise every 30 seconds
      return hasPending ? 5000 : 30000;
    },
    staleTime: 0,
  });

  // Use only backend data (no local storage)
  const filteredIntents = useMemo(() => {
    const backend = backendData?.data || [];

    // Apply client-side filters
    let filtered = backend;

    // Apply client-side status filter
    if (filters?.status) {
      filtered = filtered.filter(intent => intent.status === filters.status);
    }

    // Apply client-side chain filter
    if (filters?.chain) {
      filtered = filtered.filter(intent =>
        intent.source_chain.toLowerCase().includes(filters.chain!) ||
        intent.dest_chain.toLowerCase().includes(filters.chain!)
      );
    }

    // Apply limit if specified
    if (filters?.limit && filtered.length > filters.limit) {
      filtered = filtered.slice(0, filters.limit);
    }

    return filtered;
  }, [backendData, filters?.status, filters?.chain, filters?.limit]);

  return {
    intents: filteredIntents,
    count: backendData?.count || filteredIntents.length,
    isLoading,
    error: error instanceof Error ? error.message : null,
    refetch,
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
 * Handles chain IDs, names, and various formats
 */
export function formatChainName(chain: string): string {
  const normalized = chain.toString().toLowerCase();

  // Handle chain IDs
  if (normalized === "11155111" || normalized === "sepolia") {
    return "Ethereum Sepolia";
  }
  if (normalized === "5003") {
    return "Mantle Sepolia";
  }
  if (normalized === "1" || normalized === "mainnet") {
    return "Ethereum";
  }
  if (normalized === "5000") {
    return "Mantle";
  }

  // Handle string names
  if (normalized.includes("ethereum")) {
    return normalized.includes("sepolia") ? "Ethereum Sepolia" : "Ethereum";
  }
  if (normalized.includes("mantle")) {
    return normalized.includes("sepolia") ? "Mantle Sepolia" : "Mantle";
  }

  // Return as-is if unknown
  return chain;
}

/**
 * Format token amount for display
 * Defaults to 6 decimals for USDC/USDT
 */
export function formatAmount(amount: string, decimals: number = 6): string {
  try {
    const value = BigInt(amount);
    const divisor = BigInt(10 ** decimals);
    const quotient = value / divisor;
    const remainder = value % divisor;

    if (remainder === BigInt(0)) {
      return quotient.toString();
    }

    const remainderStr = remainder.toString().padStart(decimals, "0");
    const trimmedRemainder = remainderStr.slice(0, Math.min(6, decimals)).replace(/0+$/, "");

    if (trimmedRemainder === "") {
      return quotient.toString();
    }

    return `${quotient}.${trimmedRemainder}`;
  } catch (error) {
    console.error("Error formatting amount:", error);
    return amount;
  }
}
