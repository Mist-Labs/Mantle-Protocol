/**
 * Intent Pool Contract Watcher
 *
 * Watches IntentCreated events on Intent Pool contracts to trigger real-time UI updates
 * Only watches the currently connected chain (cannot watch both chains simultaneously)
 */

import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useChainId, useWatchContractEvent } from 'wagmi'
import { INTENT_POOL_ABI, ETHEREUM_CONTRACTS, MANTLE_CONTRACTS } from '@/lib/contracts'

/**
 * Hook to watch IntentCreated events on the currently connected chain's Intent Pool
 * Invalidates React Query cache when new intents are created
 */
export function useIntentPoolWatch() {
  const queryClient = useQueryClient()
  const chainId = useChainId()

  // Determine which Intent Pool to watch based on connected chain
  const intentPoolAddress = chainId === 11155111
    ? ETHEREUM_CONTRACTS.intentPool
    : chainId === 5003
    ? MANTLE_CONTRACTS.intentPool
    : undefined

  // Watch IntentCreated events on the current chain
  useWatchContractEvent({
    address: intentPoolAddress,
    abi: INTENT_POOL_ABI,
    eventName: 'IntentCreated',
    enabled: !!intentPoolAddress,
    onLogs(logs) {
      console.log('🔔 IntentCreated event detected:', logs.length, 'new intent(s)')

      // Invalidate bridge intents queries to trigger refetch
      queryClient.invalidateQueries({
        queryKey: ['bridgeIntents']
      })

      // Invalidate specific intent queries if needed
      logs.forEach((log) => {
        const intentId = log.args.intentId
        if (intentId) {
          queryClient.invalidateQueries({
            queryKey: ['intentStatus', intentId]
          })
        }
      })
    },
    // Poll every block for new events
    pollingInterval: chainId === 5003 ? 3000 : 12000, // Mantle ~3s, Ethereum ~12s
  })

  return {
    watchingChain: chainId === 11155111 ? 'ethereum' : chainId === 5003 ? 'mantle' : 'unknown',
    isWatching: !!intentPoolAddress,
  }
}
