"use client"

import { useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { useAccount, useBalance, useChainId, useSwitchChain } from "wagmi"
import { parseEther, formatEther } from "viem"
import { toast } from "sonner"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ArrowDownUp, ArrowRight, Shield, Clock, DollarSign, AlertCircle, CheckCircle2 } from "lucide-react"
import BridgeProgress from "./BridgeProgress"

// Supported networks
const NETWORKS = [
  { id: 5000, name: "Mantle Mainnet", symbol: "MNT", logo: "M" },
  { id: 5003, name: "Mantle Sepolia", symbol: "MNT", logo: "M" },
  { id: 1, name: "Ethereum Mainnet", symbol: "ETH", logo: "Ξ" },
  { id: 11155111, name: "Ethereum Sepolia", symbol: "ETH", logo: "Ξ" },
]

export default function BridgeForm() {
  const { address, isConnected } = useAccount()
  const chainId = useChainId()
  const { switchChain } = useSwitchChain()

  // Form state
  const [fromNetwork, setFromNetwork] = useState(NETWORKS[0])
  const [toNetwork, setToNetwork] = useState(NETWORKS[1])
  const [amount, setAmount] = useState("")
  const [destinationAddress, setDestinationAddress] = useState("")
  const [useConnectedWallet, setUseConnectedWallet] = useState(true)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [showProgress, setShowProgress] = useState(false)

  // Get balance for current network
  const { data: balanceData } = useBalance({
    address: address,
    chainId: fromNetwork.id,
  })

  // Calculate fees and amounts
  const calculateFees = () => {
    if (!amount || isNaN(Number(amount))) return { fee: "0", total: "0", receive: "0" }

    const amountNum = Number(amount)
    const feePercent = 0.15 / 100 // 0.15%
    const fee = amountNum * feePercent
    const receive = amountNum - fee

    return {
      fee: fee.toFixed(6),
      total: amount,
      receive: receive.toFixed(6),
    }
  }

  const fees = calculateFees()

  // Swap networks
  const handleSwapNetworks = () => {
    const temp = fromNetwork
    setFromNetwork(toNetwork)
    setToNetwork(temp)
  }

  // Set max amount
  const handleSetMax = () => {
    if (balanceData) {
      const balance = formatEther(balanceData.value)
      const maxWithBuffer = Math.max(0, Number(balance) - 0.001).toFixed(6)
      setAmount(maxWithBuffer)
    }
  }

  // Handle bridge submit
  const handleBridge = async () => {
    // Validations
    if (!isConnected) {
      toast.error("Please connect your wallet")
      return
    }

    if (!amount || Number(amount) <= 0) {
      toast.error("Please enter an amount")
      return
    }

    if (chainId !== fromNetwork.id) {
      toast.error(`Please switch to ${fromNetwork.name} network`)
      // Optionally auto-switch
      if (switchChain) {
        switchChain({ chainId: fromNetwork.id })
      }
      return
    }

    const destination = useConnectedWallet ? address : destinationAddress
    if (!destination) {
      toast.error("Please enter a destination address")
      return
    }

    // Start bridge process
    setIsSubmitting(true)
    setShowProgress(true)

    try {
      // This is where you would call the actual bridge function
      // For now, we'll simulate the process
      toast.success("Bridge initiated successfully!")
    } catch (error) {
      console.error("Bridge error:", error)
      toast.error("Bridge failed. Please try again.")
      setShowProgress(false)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <>
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5 }}>
        <Card className="mx-auto max-w-2xl border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm">
          {/* Header */}
          <div className="mb-6">
            <h2 className="mb-2 text-2xl font-bold text-white">Bridge Assets</h2>
            <p className="text-sm text-neutral-400">Transfer tokens across chains privately and instantly</p>
          </div>

          {/* Network Selection */}
          <div className="mb-6 space-y-4">
            {/* From Network */}
            <div>
              <Label className="mb-2 text-neutral-300">From</Label>
              <Select
                value={fromNetwork.id.toString()}
                onValueChange={(value) => setFromNetwork(NETWORKS.find((n) => n.id === parseInt(value))!)}
              >
                <SelectTrigger className="h-20 border-neutral-700 bg-neutral-800 py-4 text-white">
                  <SelectValue>
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10">
                        <span className="text-sm font-bold text-orange-500">{fromNetwork.logo}</span>
                      </div>
                      <div className="text-left">
                        <div className="font-medium text-white">{fromNetwork.name}</div>
                        <div className="text-xs text-neutral-500">
                          Balance: {balanceData ? formatEther(balanceData.value).slice(0, 8) : "0"} {fromNetwork.symbol}
                        </div>
                      </div>
                    </div>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {NETWORKS.map((network) => (
                    <SelectItem key={network.id} value={network.id.toString()}>
                      <div className="flex items-center gap-3">
                        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10">
                          <span className="text-sm font-bold text-orange-500">{network.logo}</span>
                        </div>
                        <span>{network.name}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Swap Button */}
            <div className="relative z-10 -my-2 flex justify-center">
              <button
                onClick={handleSwapNetworks}
                className="flex h-10 w-10 items-center justify-center rounded-full border border-neutral-700 bg-neutral-800 transition-all duration-300 hover:rotate-180 hover:border-orange-500"
              >
                <ArrowDownUp className="h-5 w-5 text-orange-500" />
              </button>
            </div>

            {/* To Network */}
            <div>
              <Label className="mb-2 text-neutral-300">To</Label>
              <Select
                value={toNetwork.id.toString()}
                onValueChange={(value) => setToNetwork(NETWORKS.find((n) => n.id === parseInt(value))!)}
              >
                <SelectTrigger className="h-20 border-neutral-700 bg-neutral-800 py-4 text-white">
                  <SelectValue>
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10">
                        <span className="text-sm font-bold text-orange-500">{toNetwork.logo}</span>
                      </div>
                      <div className="text-left">
                        <div className="font-medium text-white">{toNetwork.name}</div>
                        <div className="text-xs text-neutral-500">Destination chain</div>
                      </div>
                    </div>
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {NETWORKS.filter((n) => n.id !== fromNetwork.id).map((network) => (
                    <SelectItem key={network.id} value={network.id.toString()}>
                      <div className="flex items-center gap-3">
                        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10">
                          <span className="text-sm font-bold text-orange-500">{network.logo}</span>
                        </div>
                        <span>{network.name}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Amount Input */}
          <div className="mb-6">
            <div className="mb-2 flex items-center justify-between">
              <Label className="text-neutral-300">Amount</Label>
              <button
                onClick={handleSetMax}
                className="text-xs text-orange-500 transition-colors hover:text-orange-400"
              >
                MAX
              </button>
            </div>
            <div className="relative">
              <Input
                type="number"
                placeholder="0.0"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="h-16 border-neutral-700 bg-neutral-800 pr-20 text-2xl font-bold text-white"
              />
              <div className="absolute right-4 top-1/2 -translate-y-1/2 transform text-neutral-500">
                {fromNetwork.symbol}
              </div>
            </div>
            {amount && Number(amount) > 0 && <div className="mt-2 text-sm text-neutral-500">≈ $--.- USD</div>}
          </div>

          {/* Destination Address */}
          <div className="mb-6">
            <Label className="mb-2 text-neutral-300">Destination Address</Label>
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="useConnected"
                  checked={useConnectedWallet}
                  onChange={(e) => setUseConnectedWallet(e.target.checked)}
                  className="h-4 w-4 rounded border-neutral-700 bg-neutral-800 text-orange-500"
                />
                <label htmlFor="useConnected" className="cursor-pointer text-sm text-neutral-400">
                  Use connected wallet address
                </label>
              </div>
              {!useConnectedWallet && (
                <Input
                  type="text"
                  placeholder="0x..."
                  value={destinationAddress}
                  onChange={(e) => setDestinationAddress(e.target.value)}
                  className="border-neutral-700 bg-neutral-800 text-white"
                />
              )}
            </div>
          </div>

          {/* Transaction Summary */}
          {amount && Number(amount) > 0 && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              className="mb-6 space-y-3 rounded-lg border border-neutral-700 bg-neutral-800/50 p-4"
            >
              <div className="flex items-center justify-between text-sm">
                <span className="text-neutral-400">You send</span>
                <span className="font-medium text-white">
                  {amount} {fromNetwork.symbol}
                </span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="flex items-center gap-1 text-neutral-400">
                  <DollarSign className="h-3 w-3" />
                  Fee (0.15%)
                </span>
                <span className="text-neutral-300">
                  {fees.fee} {fromNetwork.symbol}
                </span>
              </div>
              <div className="flex items-center justify-between border-t border-neutral-700 pt-3">
                <span className="flex items-center gap-1 text-neutral-400">
                  <CheckCircle2 className="h-4 w-4 text-green-500" />
                  You receive
                </span>
                <span className="text-lg font-bold text-white">
                  {fees.receive} {toNetwork.symbol}
                </span>
              </div>
              <div className="flex items-center justify-between text-xs text-neutral-500">
                <span className="flex items-center gap-1">
                  <Clock className="h-3 w-3" />
                  Est. time
                </span>
                <span>10-30 seconds</span>
              </div>
              <div className="flex items-center gap-2 rounded border border-orange-500/20 bg-orange-500/10 p-2 text-xs text-neutral-500">
                <Shield className="h-4 w-4 text-orange-500" />
                <span>Privacy-enhanced with zero-knowledge commitments</span>
              </div>
            </motion.div>
          )}

          {/* Bridge Button */}
          <Button
            onClick={handleBridge}
            disabled={!isConnected || !amount || Number(amount) <= 0 || isSubmitting}
            className="h-12 w-full bg-orange-500 text-lg font-semibold text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105 hover:bg-orange-600 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:scale-100"
          >
            {!isConnected ? (
              "Connect Wallet"
            ) : chainId !== fromNetwork.id ? (
              `Switch to ${fromNetwork.name}`
            ) : isSubmitting ? (
              "Processing..."
            ) : (
              <>
                Bridge Now
                <ArrowRight className="ml-2 h-5 w-5" />
              </>
            )}
          </Button>

          {/* Info Box */}
          <div className="mt-4 flex items-start gap-2 rounded border border-neutral-700 bg-neutral-800/50 p-3 text-xs text-neutral-500">
            <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
            <p>
              Bridge transactions are privacy-preserving and auto-claimed. Funds will appear in your destination wallet
              in 10-30 seconds.
            </p>
          </div>
        </Card>
      </motion.div>

      {/* Progress Modal */}
      <AnimatePresence>
        {showProgress && (
          <BridgeProgress
            onClose={() => setShowProgress(false)}
            fromNetwork={fromNetwork.name}
            toNetwork={toNetwork.name}
            amount={amount}
            token={fromNetwork.symbol}
          />
        )}
      </AnimatePresence>
    </>
  )
}
