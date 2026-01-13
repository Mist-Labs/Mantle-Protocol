"use client"

import { useState, useEffect } from "react"
import { motion } from "framer-motion"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  CheckCircle2,
  Loader2,
  ExternalLink,
  ArrowRight,
  Shield,
  Network,
  Clock,
  FileSignature,
  CheckSquare,
  XCircle,
} from "lucide-react"
import type { Hex } from "viem"
import type { BridgeStep } from "@/hooks/useBridge"
import type { IntentStatus } from "@/lib/api"
import { getTxUrl, getChainType } from "@/lib/contracts"

interface BridgeProgressProps {
  onClose: () => void
  fromNetwork: string
  toNetwork: string
  amount: string
  token: string
  step: BridgeStep
  intentId?: Hex
  txHash?: Hex
  status?: IntentStatus
  error?: string
}

// Map bridge steps to UI display
const STEP_MAP: Record<
  BridgeStep,
  {
    title: string
    description: string
    icon: typeof Shield
  }
> = {
  idle: {
    title: "Ready",
    description: "Click Bridge Now to start",
    icon: Shield,
  },
  "generating-params": {
    title: "Generating Privacy Params",
    description: "Creating commitment and secrets...",
    icon: Shield,
  },
  "signing-auth": {
    title: "Sign Authorization",
    description: "Sign to authorize auto-claim...",
    icon: FileSignature,
  },
  "approving-token": {
    title: "Approving Token",
    description: "Approving token for bridge contract...",
    icon: CheckSquare,
  },
  "creating-intent": {
    title: "Creating Intent",
    description: "Submitting intent on-chain...",
    icon: Network,
  },
  "submitting-backend": {
    title: "Notifying Backend",
    description: "Registering with relayer...",
    icon: Network,
  },
  "waiting-solver": {
    title: "Waiting for Solver",
    description: "Solvers competing for best rate...",
    icon: Loader2,
  },
  completed: {
    title: "Complete",
    description: "Bridge successful!",
    icon: CheckCircle2,
  },
  failed: {
    title: "Failed",
    description: "Transaction failed",
    icon: XCircle,
  },
}

// Steps to display in order
const DISPLAY_STEPS: BridgeStep[] = [
  "generating-params",
  "signing-auth",
  "creating-intent",
  "waiting-solver",
  "completed",
]

export default function BridgeProgress({
  onClose,
  fromNetwork,
  toNetwork,
  amount,
  token,
  step,
  intentId,
  txHash,
  status,
  error,
}: BridgeProgressProps) {
  // Status is now coming from React Query (realtime), so trust it over step
  const isComplete = status === "completed" || step === "completed"
  const isFailed = status === "failed" || status === "refunded" || step === "failed"
  const isFullyCompleted = status === "completed"
  const isWaitingForSolver = !isFullyCompleted && !isFailed && (status === "committed" || status === "created" || status === "filled" || step === "waiting-solver")

  // Calculate progress
  const currentStepIndex = DISPLAY_STEPS.indexOf(step)
  const progress = isFullyCompleted
    ? 100 // Show 100% when completed
    : currentStepIndex >= 0
      ? ((currentStepIndex + 1) / DISPLAY_STEPS.length) * 100
      : 0

  // Get explorer URL for tx hash
  const getExplorerUrl = () => {
    if (!txHash) return null

    // Determine which chain the tx is on (source chain for creation)
    const chainType = getChainType(fromNetwork.includes("Ethereum") ? 11155111 : 5003)
    if (!chainType) return null

    return getTxUrl(chainType, txHash)
  }

  const explorerUrl = getExplorerUrl()

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="max-h-[85vh] max-w-lg overflow-hidden border border-neutral-800 bg-neutral-900 text-white">
        <DialogHeader className="border-b border-neutral-800 pb-4">
          <DialogTitle className="text-xl font-bold">
            {isFullyCompleted
              ? "Bridge Complete!"
              : isWaitingForSolver && !isFailed
                ? "Transaction Submitted!"
                : isFailed
                  ? "Bridge Failed"
                  : "Processing Bridge..."}
          </DialogTitle>
          {/* Progress Bar - moved to header */}
          <div className="mt-3">
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-neutral-800">
              <motion.div
                className={`h-full ${
                  isFailed
                    ? "bg-gradient-to-r from-red-500 to-orange-500"
                    : "bg-gradient-to-r from-orange-500 to-pink-500"
                }`}
                initial={{ width: 0 }}
                animate={{ width: `${progress}%` }}
                transition={{ duration: 0.5 }}
              />
            </div>
            <div className="mt-1.5 flex justify-between text-xs text-neutral-500">
              <span>
                Step {Math.max(currentStepIndex + 1, 1)} of {DISPLAY_STEPS.length}
              </span>
              <span>{Math.round(progress)}%</span>
            </div>
          </div>
        </DialogHeader>

        {/* Scrollable content area */}
        <div className="max-h-[calc(85vh-140px)] space-y-4 overflow-y-auto px-1 pb-4">
          {/* Transaction Details - Simplified */}
          <div className="grid grid-cols-3 gap-3 rounded-lg border border-neutral-700/50 bg-neutral-800/30 p-3">
            <div className="text-center">
              <div className="mb-1 text-xs text-neutral-500">From</div>
              <div className="truncate text-sm font-medium text-white">{fromNetwork}</div>
            </div>
            <div className="flex items-center justify-center">
              <ArrowRight className="h-4 w-4 text-orange-500" />
            </div>
            <div className="text-center">
              <div className="mb-1 text-xs text-neutral-500">To</div>
              <div className="truncate text-sm font-medium text-white">{toNetwork}</div>
            </div>
            <div className="col-span-3 border-t border-neutral-700/50 pt-2 text-center">
              <div className="mb-1 text-xs text-neutral-500">Amount</div>
              <div className="text-base font-bold text-white">
                {amount} {token}
              </div>
            </div>
          </div>

          {/* Success Message - Transaction Submitted (waiting for solver) */}
          {isWaitingForSolver && !isFailed && !error && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              className="rounded-lg border border-blue-500/50 bg-blue-500/10 p-4"
            >
              <div className="mb-2 flex items-center gap-2 font-semibold text-blue-400">
                <Clock className="h-5 w-5 flex-shrink-0" />
                <span>Transaction Submitted Successfully</span>
              </div>
              <p className="text-sm leading-relaxed text-neutral-300">
                Your transaction is on-chain and being processed by solvers. This typically takes 10-60 seconds.
                The modal will close automatically when complete.
              </p>
            </motion.div>
          )}

          {/* Success Message - Fully Complete */}
          {isFullyCompleted && !error && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              className="rounded-lg border border-green-500/50 bg-green-500/10 p-4"
            >
              <div className="mb-2 flex items-center gap-2 font-semibold text-green-400">
                <CheckCircle2 className="h-5 w-5 flex-shrink-0" />
                <span>Bridge Complete!</span>
              </div>
              <p className="text-sm leading-relaxed text-neutral-300">
                Your funds have been successfully bridged and are now available on {toNetwork}.
              </p>
            </motion.div>
          )}

          {/* Error Message */}
          {error && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              className="rounded-lg border border-red-500/50 bg-red-500/10 p-4"
            >
              <div className="mb-2 flex items-center gap-2 font-semibold text-red-400">
                <XCircle className="h-5 w-5 flex-shrink-0" />
                <span>Transaction Failed</span>
              </div>
              <p className="text-sm leading-relaxed text-neutral-300">{error}</p>
            </motion.div>
          )}

          {/* Steps */}
          <div className="space-y-2">
            {DISPLAY_STEPS.map((displayStep, index) => {
              const stepInfo = STEP_MAP[displayStep]
              const Icon = stepInfo.icon
              const isActive = step === displayStep
              const isCompleted = currentStepIndex > index
              const isPending = currentStepIndex < index

              return (
                <motion.div
                  key={displayStep}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.05 }}
                  className={`flex items-center gap-3 rounded-lg border p-3 transition-all ${
                    isActive
                      ? "border-orange-500/40 bg-orange-500/10"
                      : isCompleted
                        ? "border-green-500/40 bg-green-500/5"
                        : "border-neutral-700/50 bg-neutral-800/20"
                  }`}
                >
                  <div
                    className={`flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full ${
                      isActive ? "bg-orange-500/20" : isCompleted ? "bg-green-500/20" : "bg-neutral-700/20"
                    }`}
                  >
                    {isActive ? (
                      <Loader2 className="h-4.5 w-4.5 animate-spin text-orange-500" />
                    ) : isCompleted ? (
                      <CheckCircle2 className="h-4.5 w-4.5 text-green-500" />
                    ) : (
                      <Icon className="h-4.5 w-4.5 text-neutral-500" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div
                      className={`text-sm font-semibold ${
                        isActive ? "text-orange-400" : isCompleted ? "text-green-400" : "text-neutral-400"
                      }`}
                    >
                      {stepInfo.title}
                    </div>
                    <div className="truncate text-xs text-neutral-500">{stepInfo.description}</div>
                  </div>
                </motion.div>
              )
            })}
          </div>

          {/* Transaction Hash & Intent ID */}
          {(txHash || intentId) && (
            <div className="space-y-2">
              {txHash && (
                <motion.div
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="rounded-lg border border-neutral-700/50 bg-neutral-800/30 p-3"
                >
                  <div className="mb-2 flex items-center justify-between">
                    <div className="flex items-center gap-2 text-xs font-medium text-neutral-400">
                      <Network className="h-3.5 w-3.5 text-orange-500" />
                      Transaction Hash
                    </div>
                    {explorerUrl && (
                      <a
                        href={explorerUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-1 rounded-md bg-orange-500/10 px-2 py-1 text-xs font-medium text-orange-500 transition-all hover:bg-orange-500/20"
                      >
                        View
                        <ExternalLink className="h-3 w-3" />
                      </a>
                    )}
                  </div>
                  <div className="truncate font-mono text-xs text-neutral-300">{txHash}</div>
                </motion.div>
              )}

              {intentId && (
                <motion.div
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="rounded-lg border border-neutral-700/50 bg-neutral-800/30 p-3"
                >
                  <div className="mb-2 flex items-center gap-2 text-xs font-medium text-neutral-400">
                    <Shield className="h-3.5 w-3.5 text-orange-500" />
                    Intent ID
                  </div>
                  <div className="truncate font-mono text-xs text-neutral-300">{intentId}</div>
                </motion.div>
              )}
            </div>
          )}

          {/* Waiting Message / Actions */}
          {!isComplete && !isFailed && (
            <div className="rounded-lg border border-neutral-700/50 bg-neutral-800/20 p-4 text-center">
              <div className="flex items-center justify-center gap-2 text-sm text-neutral-400">
                <Clock className="h-4 w-4 animate-pulse text-orange-500" />
                <span>Please wait... This usually takes 10-30 seconds</span>
              </div>
            </div>
          )}
        </div>

        {/* Actions Footer - Fixed at bottom */}
        {(isComplete || isFailed) && (
          <div className="border-t border-neutral-800 pt-4">
            {isComplete ? (
              <div className="flex gap-3">
                <Button className="flex-1 bg-orange-500 hover:bg-orange-600" onClick={onClose}>
                  Bridge Again
                </Button>
                <Button
                  variant="outline"
                  className="flex-1 border-neutral-700 bg-neutral-800 hover:bg-neutral-700"
                  onClick={() => {
                    window.location.href = "/activity"
                  }}
                >
                  View Activity
                </Button>
              </div>
            ) : (
              <Button
                variant="outline"
                className="w-full border-neutral-700 bg-neutral-800 hover:bg-neutral-700"
                onClick={onClose}
              >
                Close
              </Button>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
