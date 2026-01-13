/**
 * Transaction History Hook
 *
 * Manages bridge transaction history in local storage
 * Provides methods to add, update, and query transactions
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import type { Hex } from "viem";
import type { IntentStatus } from "@/lib/api";
import type { ChainType } from "@/lib/tokens";

/**
 * Stored transaction record
 */
export interface BridgeTransaction {
  intentId: Hex;
  txHash: Hex;
  userAddress: string;
  sourceChain: ChainType;
  destChain: ChainType;
  sourceToken: string;
  destToken: string;
  amount: string;
  status: IntentStatus;
  timestamp: number;
  lastUpdated: number;
  completedAt?: number;
}

const STORAGE_KEY = "shadow_swap_transactions";
const MAX_TRANSACTIONS = 100; // Keep last 100 transactions

/**
 * Load transactions from local storage
 */
function loadTransactions(): BridgeTransaction[] {
  if (typeof window === "undefined") return [];

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return [];

    const transactions = JSON.parse(stored) as BridgeTransaction[];
    return transactions.sort((a, b) => b.timestamp - a.timestamp);
  } catch (error) {
    console.error("Failed to load transactions:", error);
    return [];
  }
}

/**
 * Save transactions to local storage
 */
function saveTransactions(transactions: BridgeTransaction[]): void {
  if (typeof window === "undefined") return;

  try {
    // Keep only the most recent MAX_TRANSACTIONS
    const trimmed = transactions
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, MAX_TRANSACTIONS);

    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch (error) {
    console.error("Failed to save transactions:", error);
  }
}

/**
 * Hook for managing transaction history
 */
export function useTransactionHistory(userAddress?: string) {
  const [transactions, setTransactions] = useState<BridgeTransaction[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Load transactions on mount
  useEffect(() => {
    const loaded = loadTransactions();
    setTransactions(loaded);
    setIsLoading(false);
  }, []);

  /**
   * Add a new transaction
   */
  const addTransaction = useCallback(
    (transaction: Omit<BridgeTransaction, "timestamp" | "lastUpdated">) => {
      const newTransaction: BridgeTransaction = {
        ...transaction,
        timestamp: Date.now(),
        lastUpdated: Date.now(),
      };

      setTransactions((prev) => {
        const updated = [newTransaction, ...prev];
        saveTransactions(updated);
        return updated;
      });

      return newTransaction;
    },
    []
  );

  /**
   * Update an existing transaction
   */
  const updateTransaction = useCallback(
    (intentId: Hex, updates: Partial<BridgeTransaction>) => {
      setTransactions((prev) => {
        const index = prev.findIndex((tx) => tx.intentId === intentId);
        if (index === -1) return prev;

        const updated = [...prev];
        updated[index] = {
          ...updated[index],
          ...updates,
          lastUpdated: Date.now(),
          completedAt:
            updates.status === "completed" ? Date.now() : updated[index].completedAt,
        };

        saveTransactions(updated);
        return updated;
      });
    },
    []
  );

  /**
   * Get transaction by intent ID
   */
  const getTransaction = useCallback(
    (intentId: Hex): BridgeTransaction | undefined => {
      return transactions.find((tx) => tx.intentId === intentId);
    },
    [transactions]
  );

  /**
   * Delete a transaction
   */
  const deleteTransaction = useCallback((intentId: Hex) => {
    setTransactions((prev) => {
      const updated = prev.filter((tx) => tx.intentId !== intentId);
      saveTransactions(updated);
      return updated;
    });
  }, []);

  /**
   * Clear all transactions
   */
  const clearAll = useCallback(() => {
    setTransactions([]);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  /**
   * Filter transactions by user address
   */
  const userTransactions = useMemo(() => {
    if (!userAddress) return transactions;
    return transactions.filter(
      (tx) => tx.userAddress.toLowerCase() === userAddress.toLowerCase()
    );
  }, [transactions, userAddress]);

  /**
   * Get transactions by status
   */
  const getByStatus = useCallback(
    (status: IntentStatus) => {
      return userTransactions.filter((tx) => tx.status === status);
    },
    [userTransactions]
  );

  /**
   * Get pending transactions
   */
  const pendingTransactions = useMemo(() => {
    return userTransactions.filter(
      (tx) =>
        tx.status === "committed" ||
        tx.status === "filled" ||
        tx.status === "created"
    );
  }, [userTransactions]);

  /**
   * Get completed transactions
   */
  const completedTransactions = useMemo(() => {
    return userTransactions.filter((tx) => tx.status === "completed");
  }, [userTransactions]);

  /**
   * Get failed transactions
   */
  const failedTransactions = useMemo(() => {
    return userTransactions.filter((tx) => tx.status === "failed");
  }, [userTransactions]);

  /**
   * Statistics
   */
  const stats = useMemo(() => {
    return {
      total: userTransactions.length,
      pending: pendingTransactions.length,
      completed: completedTransactions.length,
      failed: failedTransactions.length,
    };
  }, [userTransactions, pendingTransactions, completedTransactions, failedTransactions]);

  return {
    transactions: userTransactions,
    allTransactions: transactions,
    isLoading,
    stats,
    pendingTransactions,
    completedTransactions,
    failedTransactions,
    addTransaction,
    updateTransaction,
    getTransaction,
    getByStatus,
    deleteTransaction,
    clearAll,
  };
}
