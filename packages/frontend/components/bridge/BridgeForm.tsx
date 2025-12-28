"use client"

import { useState, useEffect } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { useAccount, useBalance, useChainId, useSwitchChain } from "wagmi"
import { parseEther, formatEther } from "viem"
import type { Address } from "viem"
import { toast } from "sonner"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ArrowDownUp, ArrowRight, Shield, Clock, DollarSign, AlertCircle, CheckCircle2, Loader2 } from "lucide-react"
import BridgeProgress from "./BridgeProgress"
import { useBridge } from "@/hooks/useBridge"
import { SUPPORTED_TOKENS } from "@/lib/tokens"
import { useTokenUSDValue, useTokenConversion } from "@/hooks/usePriceFeed"

// Supported networks (testnets only for MVP)
const NETWORKS = [
  { id: 5003, name: "Mantle Sepolia", symbol: "MNT", logo: "M", chain: "mantle" as const },
  { id: 11155111, name: "Ethereum Sepolia", symbol: "ETH", logo: "Ξ", chain: "ethereum" as const },
]

export default function BridgeForm() {
  const { address, isConnected } = useAccount()
  const chainId = useChainId()
  const { switchChain } = useSwitchChain()

  // Bridge hook
  const { bridge, reset, state, isLoading, step, intentId, txHash, status, error } = useBridge()

  // Form state
  const [fromNetwork, setFromNetwork] = useState(NETWORKS[0])
  const [toNetwork, setToNetwork] = useState(NETWORKS[1])
  const [selectedToken, setSelectedToken] = useState<string>("ETH")
  const [amount, setAmount] = useState("")
  const [destinationAddress, setDestinationAddress] = useState("")
  const [useConnectedWallet, setUseConnectedWallet] = useState(true)
  const [showProgress, setShowProgress] = useState(false)

  // Price feed hooks
  const { usdValue, isLoading: isLoadingUSD, price } = useTokenUSDValue(selectedToken, amount)
  const {
    outputAmount,
    rate,
    isLoading: isLoadingConversion,
  } = useTokenConversion(selectedToken, selectedToken, amount)

  // Show progress modal when bridge starts
  useEffect(() => {
    if (isLoading && !showProgress) {
      setShowProgress(true)
    }
  }, [isLoading, showProgress])

  // Close progress modal when bridge completes or fails
  useEffect(() => {
    if (!isLoading && (step === "completed" || step === "failed") && showProgress) {
      // Keep modal open to show final status
      setTimeout(() => {
        setShowProgress(false)
        reset()
      }, 5000) // Auto-close after 5 seconds
    }
  }, [isLoading, step, showProgress, reset])

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

    // Validate destination address
    if (!destination.startsWith("0x") || destination.length !== 42) {
      toast.error("Invalid destination address")
      return
    }

    try {
      // Execute bridge transaction
      await bridge({
        sourceChain: fromNetwork.chain,
        destChain: toNetwork.chain,
        tokenSymbol: selectedToken,
        amount,
        recipient: destination as Address,
      })

      toast.success("Bridge completed successfully!")

      // Reset form
      setAmount("")
      setDestinationAddress("")
    } catch (error: unknown) {
      console.error("Bridge error:", error)
      const errorMessage = error instanceof Error ? error.message : "Bridge failed"
      toast.error(errorMessage)
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
            {amount && Number(amount) > 0 && (
              <div className="mt-2 flex items-center gap-2 text-sm text-neutral-500">
                {isLoadingUSD ? (
                  <>
                    <Loader2 className="h-3 w-3 animate-spin" />
                    <span>Fetching price...</span>
                  </>
                ) : usdValue ? (
                  <>≈ ${usdValue.toFixed(2)} USD</>
                ) : (
                  <>≈ $-- USD</>
                )}
              </div>
            )}
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
                <div className="text-right">
                  <div className="font-medium text-white">
                    {amount} {fromNetwork.symbol}
                  </div>
                  {usdValue && !isLoadingUSD && (
                    <div className="text-xs text-neutral-500">≈ ${usdValue.toFixed(2)}</div>
                  )}
                </div>
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
                <div className="text-right">
                  <div className="text-lg font-bold text-white">
                    {fees.receive} {toNetwork.symbol}
                  </div>
                  {isLoadingConversion && (
                    <div className="flex items-center justify-end gap-1 text-xs text-neutral-500">
                      <Loader2 className="h-3 w-3 animate-spin" />
                      <span>Calculating...</span>
                    </div>
                  )}
                </div>
              </div>
              {price && !isLoadingUSD && (
                <div className="flex items-center justify-between text-xs text-neutral-500">
                  <span>Exchange rate</span>
                  <span>
                    1 {selectedToken} = ${price.toFixed(2)} USD
                  </span>
                </div>
              )}
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
            disabled={!isConnected || !amount || Number(amount) <= 0 || isLoading}
            className="h-12 w-full bg-orange-500 text-lg font-semibold text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105 hover:bg-orange-600 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:scale-100"
          >
            {!isConnected ? (
              "Connect Wallet"
            ) : chainId !== fromNetwork.id ? (
              `Switch to ${fromNetwork.name}`
            ) : isLoading ? (
              <>
                {step === "generating-params" && "Generating privacy params..."}
                {step === "signing-auth" && "Sign authorization..."}
                {step === "approving-token" && "Approving token..."}
                {step === "creating-intent" && "Creating intent..."}
                {step === "submitting-backend" && "Submitting to backend..."}
                {step === "waiting-solver" && "Waiting for solver..."}
              </>
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
            onClose={() => {
              setShowProgress(false)
              reset()
            }}
            fromNetwork={fromNetwork.name}
            toNetwork={toNetwork.name}
            amount={amount}
            token={selectedToken}
            step={step}
            intentId={intentId || undefined}
            txHash={txHash || undefined}
            status={status || undefined}
            error={error || undefined}
          />
        )}
      </AnimatePresence>
    </>
  )
}
