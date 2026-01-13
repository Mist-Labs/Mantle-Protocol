/**
 * Intent Status Hook
 *
 * Real-time intent status tracking using React Query
 * This is what Recent Activity uses - fast and reactive!
 */

import { useQuery } from '@tanstack/react-query'
import type { Hex } from 'viem'
import { getIntentStatus, type IntentStatusResponse } from '@/lib/api'

interface UseIntentStatusOptions {
  intentId: Hex | null | undefined
  enabled?: boolean
  /**
   * Polling interval in milliseconds
   * Set to false to disable polling
   * Default: 5000ms (5 seconds)
   */
  refetchInterval?: number | false
}

/**
 * Hook to watch a single intent's status in real-time using React Query
 * This approach is faster than Promise-based polling because it leverages React Query's cache
 */
export function useIntentStatus({
  intentId,
  enabled = true,
  refetchInterval = 5000,
}: UseIntentStatusOptions) {
  const query = useQuery({
    queryKey: ['intentStatus', intentId],
    queryFn: async () => {
      if (!intentId) return null
      return await getIntentStatus(intentId)
    },
    enabled: enabled && !!intentId,
    refetchInterval: (query) => {
      const data = query.state.data
      if (!data) return refetchInterval

      // Stop polling if terminal state reached
      const terminalStates = ['completed', 'refunded', 'failed']
      if (terminalStates.includes(data.status)) {
        return false // Stop polling
      }

      // Continue polling for pending states
      return refetchInterval
    },
    staleTime: 0, // Always consider stale to enable frequent refetching
  })

  const isTerminal =
    query.data?.status === 'completed' ||
    query.data?.status === 'refunded' ||
    query.data?.status === 'failed'

  const isPending =
    query.data?.status === 'committed' ||
    query.data?.status === 'created' ||
    query.data?.status === 'filled'

  return {
    status: query.data,
    intentStatus: query.data?.status,
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    isTerminal,
    isPending,
    error: query.error,
    refetch: query.refetch,
  }
}
