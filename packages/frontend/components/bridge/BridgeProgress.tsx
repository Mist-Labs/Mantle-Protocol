"use client"

import { useState, useEffect } from "react"
import { motion } from "framer-motion"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { CheckCircle2, Loader2, ExternalLink, ArrowRight, Shield, Network, Clock } from "lucide-react"

interface BridgeProgressProps {
  onClose: () => void
  fromNetwork: string
  toNetwork: string
  amount: string
  token: string
}

const STEPS = [
  { id: 0, title: "Creating Intent", description: "Generating privacy commitment...", icon: Shield },
  { id: 1, title: "Waiting for Solver", description: "Solvers competing for best rate...", icon: Network },
  { id: 2, title: "Solver Filled", description: "Liquidity provided on destination...", icon: CheckCircle2 },
  { id: 3, title: "Auto-Claiming", description: "Sending funds to your wallet...", icon: Clock },
  { id: 4, title: "Complete", description: "Bridge successful!", icon: CheckCircle2 },
]

export default function BridgeProgress({ onClose, fromNetwork, toNetwork, amount, token }: BridgeProgressProps) {
  const [currentStep, setCurrentStep] = useState(0)
  const [txHash, setTxHash] = useState("0x1234...5678")

  // Simulate progress
  useEffect(() => {
    const timers = [
      setTimeout(() => setCurrentStep(1), 2000),
      setTimeout(() => setCurrentStep(2), 5000),
      setTimeout(() => setCurrentStep(3), 8000),
      setTimeout(() => setCurrentStep(4), 11000),
    ]

    return () => timers.forEach(clearTimeout)
  }, [])

  const isComplete = currentStep === 4
  const progress = ((currentStep + 1) / STEPS.length) * 100

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="max-w-md border border-neutral-800 bg-neutral-900 text-white">
        <DialogHeader>
          <DialogTitle className="text-2xl font-bold">
            {isComplete ? "Bridge Complete!" : "Processing Bridge..."}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-6">
          {/* Progress Bar */}
          <div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-neutral-800">
              <motion.div
                className="h-full bg-gradient-to-r from-orange-500 to-pink-500"
                initial={{ width: 0 }}
                animate={{ width: `${progress}%` }}
                transition={{ duration: 0.5 }}
              />
            </div>
            <div className="mt-2 flex justify-between text-xs text-neutral-500">
              <span>
                Step {currentStep + 1} of {STEPS.length}
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
          </div>

          {/* Steps */}
          <div className="space-y-3">
            {STEPS.map((step) => {
              const Icon = step.icon
              const isActive = step.id === currentStep
              const isCompleted = step.id < currentStep
              const isPending = step.id > currentStep

              return (
                <motion.div
                  key={step.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: step.id * 0.1 }}
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
                      {step.title}
                    </div>
                    <div className="text-xs text-neutral-500">{step.description}</div>
                  </div>
                </motion.div>
              )
            })}
          </div>

          {/* Transaction Hash */}
          {currentStep >= 2 && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="rounded-lg border border-neutral-700 bg-neutral-800/50 p-3"
            >
              <div className="mb-1 text-xs text-neutral-400">Transaction Hash</div>
              <div className="flex items-center gap-2">
                <span className="flex-1 truncate font-mono text-sm text-white">{txHash}</span>
                <button className="text-orange-500 transition-colors hover:text-orange-400">
                  <ExternalLink className="h-4 w-4" />
                </button>
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
                    // Navigate to activity
                  }}
                >
                  View Activity
                </Button>
              </>
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
