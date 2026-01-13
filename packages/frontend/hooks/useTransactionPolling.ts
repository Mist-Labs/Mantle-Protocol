/**
 * Transaction Polling Hook
 *
 * Uses TanStack Query + Intent Pool event watching for real-time transaction updates
 * - Automatically polls backend for pending transaction status
 * - Refetches on IntentCreated events for instant updates
 * - Updates local transaction history when status changes
 */

import { useQuery } from "@tanstack/react-query"
import { useEffect } from "react"
import { getIntentStatus } from "@/lib/api"
import type { Hex } from "viem"
import { useTransactionHistory } from "./useTransactionHistory"
import { useIntentPoolWatch } from "./useIntentPoolWatch"

interface UseTransactionPollingProps {
  intentId: Hex | null
  enabled?: boolean
  userAddress?: string
}

/**
 * Hook to poll a single transaction status
 */
export function useTransactionPolling({
  intentId,
  enabled = true,
  userAddress
}: UseTransactionPollingProps) {
  const { updateTransaction } = useTransactionHistory(userAddress)

  // Watch Intent Pool events (handled globally by useIntentPoolWatch)
  useIntentPoolWatch()

  // Query for transaction status
  const { data: status, isLoading, error } = useQuery({
    queryKey: ["intentStatus", intentId],
    queryFn: async () => {
      if (!intentId) return null
      return await getIntentStatus(intentId)
    },
    enabled: enabled && !!intentId,
    refetchInterval: (query) => {
      // Stop polling if terminal state reached
      const data = query.state.data
      if (!data) return false
      const terminalStates = ["completed", "refunded", "failed"]
      if (terminalStates.includes(data.status)) {
        return false
      }
      // Poll every 5 seconds for pending transactions
      return 5000
    },
    staleTime: 0, // Always consider data stale to enable refetching
  })

  // Update local transaction history when status changes
  useEffect(() => {
    if (status && intentId) {
      updateTransaction(intentId, {
        status: status.status,
      })
    }
  }, [status, intentId, updateTransaction])

  return {
    status,
    isLoading,
    error,
  }
}

/**
 * Hook to poll all pending transactions for a user
 */
export function useAllPendingTransactionsPolling(userAddress?: string) {
  const { pendingTransactions, updateTransaction } = useTransactionHistory(userAddress)

  // Watch Intent Pool events (handled globally by useIntentPoolWatch)
  useIntentPoolWatch()

  // Poll each pending transaction
  const pendingIntentIds = pendingTransactions.map(tx => tx.intentId)

  // Query for all pending statuses
  const { data, isFetching } = useQuery({
    queryKey: ["allPendingIntents", userAddress],
    queryFn: async () => {
      if (!pendingIntentIds.length) return []

      const statuses = await Promise.allSettled(
        pendingIntentIds.map(intentId => getIntentStatus(intentId))
      )

      return statuses.map((result, index) => ({
        intentId: pendingIntentIds[index],
        status: result.status === "fulfilled" ? result.value : null,
        error: result.status === "rejected" ? result.reason : null,
      }))
    },
    enabled: !!userAddress && pendingIntentIds.length > 0,
    refetchInterval: 5000, // Poll every 5 seconds
    staleTime: 0,
  })

  // Update transaction history when statuses change
  useEffect(() => {
    if (data) {
      data.forEach(({ intentId, status }) => {
        if (status) {
          updateTransaction(intentId, {
            status: status.status,
          })
        }
      })
    }
  }, [data, updateTransaction])

  return {
    pendingCount: pendingIntentIds.length,
    isPolling: isFetching,
  }
}
