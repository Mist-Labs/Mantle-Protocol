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
  const isComplete = step === "completed"
  const isFailed = step === "failed"

  // Calculate progress
  const currentStepIndex = DISPLAY_STEPS.indexOf(step)
  const progress =
    currentStepIndex >= 0
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
      <DialogContent className="max-w-md border border-neutral-800 bg-neutral-900 text-white">
        <DialogHeader>
          <DialogTitle className="text-2xl font-bold">
            {isComplete ? "Bridge Complete!" : isFailed ? "Bridge Failed" : "Processing Bridge..."}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-6">
          {/* Progress Bar */}
          <div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-neutral-800">
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
            <div className="mt-2 flex justify-between text-xs text-neutral-500">
              <span>
                Step {Math.max(currentStepIndex + 1, 1)} of {DISPLAY_STEPS.length}
              </span>
              <span>{Math.round(progress)}%</span>
            </div>
          </div>

          {/* Transaction Details */}
          <div className="space-y-3 rounded-lg border border-neutral-700 bg-neutral-800/50 p-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-neutral-400">From</span>
              <Badge variant="outline" className="border-neutral-700">
                {fromNetwork}
              </Badge>
            </div>
            <div className="flex items-center justify-center">
              <ArrowRight className="h-4 w-4 text-orange-500" />
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-neutral-400">To</span>
              <Badge variant="outline" className="border-neutral-700">
                {toNetwork}
              </Badge>
            </div>
            <div className="flex items-center justify-between border-t border-neutral-700 pt-3">
              <span className="text-sm text-neutral-400">Amount</span>
              <span className="font-bold text-white">
                {amount} {token}
              </span>
            </div>
            {status && (
              <div className="flex items-center justify-between border-t border-neutral-700 pt-3">
                <span className="text-sm text-neutral-400">Status</span>
                <Badge
                  variant="outline"
                  className={`border-neutral-700 ${
                    status === "completed"
                      ? "bg-green-500/10 text-green-500"
                      : status === "failed"
                        ? "bg-red-500/10 text-red-500"
                        : "bg-orange-500/10 text-orange-500"
                  }`}
                >
                  {status}
                </Badge>
              </div>
            )}
          </div>

          {/* Error Message */}
          {error && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              className="rounded-lg border border-red-500/30 bg-red-500/10 p-4"
            >
              <div className="mb-1 flex items-center gap-2 text-sm font-medium text-red-500">
                <XCircle className="h-4 w-4" />
                Error
              </div>
              <div className="text-xs text-neutral-300">{error}</div>
            </motion.div>
          )}

          {/* Steps */}
          <div className="space-y-3">
            {DISPLAY_STEPS.map((displayStep, index) => {
              const stepInfo = STEP_MAP[displayStep]
              const Icon = stepInfo.icon
              const isActive = step === displayStep
              const isCompleted = currentStepIndex > index
              const isPending = currentStepIndex < index

              return (
                <motion.div
                  key={displayStep}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.1 }}
                  className={`flex items-center gap-4 rounded-lg p-3 transition-all ${
                    isActive
                      ? "border border-orange-500/30 bg-orange-500/10"
                      : isCompleted
                        ? "border border-green-500/30 bg-green-500/10"
                        : "border border-neutral-700/30 bg-neutral-800/30"
                  }`}
                >
                  <div
                    className={`flex h-10 w-10 items-center justify-center rounded-full ${
                      isActive ? "bg-orange-500/20" : isCompleted ? "bg-green-500/20" : "bg-neutral-700/20"
                    }`}
                  >
                    {isActive ? (
                      <Loader2 className="h-5 w-5 animate-spin text-orange-500" />
                    ) : isCompleted ? (
                      <CheckCircle2 className="h-5 w-5 text-green-500" />
                    ) : (
                      <Icon className="h-5 w-5 text-neutral-500" />
                    )}
                  </div>
                  <div className="flex-1">
                    <div
                      className={`font-medium ${
                        isActive ? "text-orange-500" : isCompleted ? "text-green-500" : "text-neutral-400"
                      }`}
                    >
                      {stepInfo.title}
                    </div>
                    <div className="text-xs text-neutral-500">{stepInfo.description}</div>
                  </div>
                </motion.div>
              )
            })}
          </div>

          {/* Transaction Hash */}
          {txHash && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="rounded-lg border border-neutral-700 bg-neutral-800/50 p-3"
            >
              <div className="mb-1 text-xs text-neutral-400">Transaction Hash</div>
              <div className="flex items-center gap-2">
                <span className="flex-1 truncate font-mono text-sm text-white">{txHash}</span>
                {explorerUrl && (
                  <a
                    href={explorerUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-orange-500 transition-colors hover:text-orange-400"
                  >
                    <ExternalLink className="h-4 w-4" />
                  </a>
                )}
              </div>
            </motion.div>
          )}

          {/* Intent ID */}
          {intentId && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="rounded-lg border border-neutral-700 bg-neutral-800/50 p-3"
            >
              <div className="mb-1 text-xs text-neutral-400">Intent ID</div>
              <div className="flex items-center gap-2">
                <span className="flex-1 truncate font-mono text-sm text-white">{intentId}</span>
              </div>
            </motion.div>
          )}

          {/* Actions */}
          <div className="flex gap-3">
            {isComplete ? (
              <>
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
              </>
            ) : isFailed ? (
              <Button
                variant="outline"
                className="w-full border-neutral-700 bg-neutral-800 hover:bg-neutral-700"
                onClick={onClose}
              >
                Close
              </Button>
            ) : (
              <div className="w-full text-center text-sm text-neutral-500">
                Please wait... This usually takes 10-30 seconds
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
