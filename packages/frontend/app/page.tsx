"use client"

import { useState, useEffect } from "react"
import {
  Shield,
  Bolt,
  Coins,
  ArrowDownUp,
  Copy,
  Check,
  ChevronDown,
  Clock,
  Search,
  Filter,
  TrendingUp,
  Zap,
  CheckCircle2,
  Loader2,
  Menu,
  X,
} from "lucide-react"
import { Button } from "@/components/ui/button"

export default function ShadowSwap() {
  const [bridgeAmount, setBridgeAmount] = useState("")
  const [fromNetwork, setFromNetwork] = useState("mantle")
  const [toNetwork, setToNetwork] = useState("ethereum")
  const [useConnectedWallet, setUseConnectedWallet] = useState(true)
  const [destinationAddress, setDestinationAddress] = useState("")
  const [showProgressModal, setShowProgressModal] = useState(false)
  const [progressStep, setProgressStep] = useState(0)
  const [copiedHash, setCopiedHash] = useState(false)
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null)
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const [activeTab, setActiveTab] = useState("bridge")

  // Animated counter
  const [totalVolume, setTotalVolume] = useState(0)
  const [activeUsers, setActiveUsers] = useState(0)
  const [totalTransactions, setTotalTransactions] = useState(0)

  useEffect(() => {
    const volumeTarget = 12500000
    const usersTarget = 3421
    const transactionsTarget = 45231
    const duration = 2000

    const stepVolume = volumeTarget / (duration / 16)
    const stepUsers = usersTarget / (duration / 16)
    const stepTransactions = transactionsTarget / (duration / 16)

    let currentVolume = 0
    let currentUsers = 0
    let currentTransactions = 0

    const interval = setInterval(() => {
      currentVolume = Math.min(currentVolume + stepVolume, volumeTarget)
      currentUsers = Math.min(currentUsers + stepUsers, usersTarget)
      currentTransactions = Math.min(currentTransactions + stepTransactions, transactionsTarget)

      setTotalVolume(Math.floor(currentVolume))
      setActiveUsers(Math.floor(currentUsers))
      setTotalTransactions(Math.floor(currentTransactions))

      if (currentVolume >= volumeTarget) {
        clearInterval(interval)
      }
    }, 16)

    return () => clearInterval(interval)
  }, [])

  const handleBridgeNow = () => {
    setShowProgressModal(true)
    setProgressStep(0)

    setTimeout(() => setProgressStep(1), 1000)
    setTimeout(() => setProgressStep(2), 3000)
    setTimeout(() => setProgressStep(3), 5000)
    setTimeout(() => setProgressStep(4), 7000)
  }

  const copyToClipboard = (text: string, id: string) => {
    navigator.clipboard.writeText(text)
    if (id === "hash") {
      setCopiedHash(true)
      setTimeout(() => setCopiedHash(false), 2000)
    } else {
      setCopiedAddress(id)
      setTimeout(() => setCopiedAddress(null), 2000)
    }
  }

  const formatNumber = (num: number) => {
    if (num >= 1000000) {
      return `$${(num / 1000000).toFixed(1)}M`
    }
    return num.toLocaleString()
  }

  const swapNetworks = () => {
    const temp = fromNetwork
    setFromNetwork(toNetwork)
    setToNetwork(temp)
  }

  return (
    <div className="min-h-screen bg-black">
      {/* Navigation Header */}
      <header className="sticky top-0 z-50 h-16 bg-neutral-800 border-b border-neutral-700 backdrop-blur-lg bg-opacity-95">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 h-full flex items-center justify-between">
          <div className="flex items-center gap-8">
            <h1 className="text-orange-500 font-bold text-lg tracking-wider">SHADOW SWAP</h1>

            {/* Desktop Navigation */}
            <nav className="hidden md:flex items-center gap-6">
              {["Bridge", "Activity", "Stats", "Docs"].map((item) => (
                <button
                  key={item}
                  onClick={() => setActiveTab(item.toLowerCase())}
                  className={`text-sm py-1 border-b-2 transition-colors ${
                    activeTab === item.toLowerCase()
                      ? "border-orange-500 text-orange-500"
                      : "border-transparent text-neutral-400 hover:text-white"
                  }`}
                >
                  {item.toUpperCase()}
                </button>
              ))}
            </nav>
          </div>

          <div className="flex items-center gap-3">
            {/* Network Badge */}
            <div className="hidden sm:flex items-center gap-2 bg-neutral-900 border border-neutral-700 px-3 py-1.5 rounded text-xs">
              <div className="w-2 h-2 bg-white rounded-full animate-pulse"></div>
              <span className="text-white">MANTLE L2</span>
            </div>

            {/* Connect Wallet Button */}
            <Button className="bg-orange-500 hover:bg-orange-600 text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105">
              Connect Wallet
            </Button>

            {/* Mobile Menu */}
            <button
              className="md:hidden text-neutral-400 hover:text-white"
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            >
              {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
            </button>
          </div>
        </div>

        {/* Mobile Menu Dropdown */}
        {mobileMenuOpen && (
          <div className="md:hidden absolute top-16 left-0 right-0 bg-neutral-900 border-b border-neutral-700">
            <nav className="flex flex-col p-4 gap-2">
              {["Bridge", "Activity", "Stats", "Docs"].map((item) => (
                <button
                  key={item}
                  onClick={() => {
                    setActiveTab(item.toLowerCase())
                    setMobileMenuOpen(false)
                  }}
                  className={`text-left py-2 px-3 rounded transition-colors ${
                    activeTab === item.toLowerCase()
                      ? "bg-orange-500 text-white"
                      : "text-neutral-400 hover:text-white hover:bg-neutral-800"
                  }`}
                >
                  {item.toUpperCase()}
                </button>
              ))}
            </nav>
          </div>
        )}
      </header>

      {/* Hero Section */}
      <section className="relative py-16 sm:py-24 px-4 sm:px-6 overflow-hidden">
        {/* Grid Pattern Background */}
        <div
          className="absolute inset-0 opacity-20"
          style={{
            backgroundImage: `linear-gradient(rgba(245, 115, 22, 0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(245, 115, 22, 0.1) 1px, transparent 1px)`,
            backgroundSize: "50px 50px",
          }}
        />

        <div className="max-w-4xl mx-auto text-center relative z-10">
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold tracking-wider mb-4 text-balance">
            Bridge Assets Across Chains. <span className="text-orange-500">Privately. Instantly.</span>
          </h2>
          <p className="text-neutral-400 text-base sm:text-lg mb-8 max-w-2xl mx-auto">
            Privacy-enhanced bridging powered by intent-based architecture
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center mb-12">
            <Button
              onClick={() => setActiveTab("bridge")}
              className="bg-orange-500 hover:bg-orange-600 text-white shadow-lg shadow-orange-500/20 px-8 py-6 text-base transition-all duration-300 hover:scale-105"
            >
              Launch App
            </Button>
            <Button
              variant="outline"
              className="border-orange-500 text-orange-500 hover:bg-orange-500/10 px-8 py-6 text-base transition-all duration-300 bg-transparent"
            >
              Read Docs
            </Button>
          </div>

          {/* Stats Ticker */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
            {[
              { label: "Total Volume", value: formatNumber(totalVolume), trend: "+15.3%" },
              { label: "Active Users", value: formatNumber(activeUsers), trend: "+8.2%" },
              { label: "Avg Bridge Time", value: "18s", trend: null },
              { label: "Total Transactions", value: formatNumber(totalTransactions), trend: null },
            ].map((stat) => (
              <div
                key={stat.label}
                className="bg-neutral-900 border border-neutral-700 p-4 rounded backdrop-blur-sm bg-opacity-50 hover:border-orange-500/50 transition-all duration-300"
              >
                <div className="text-neutral-400 text-xs uppercase tracking-wider mb-1">{stat.label}</div>
                <div className="text-white text-xl sm:text-2xl font-bold font-mono">{stat.value}</div>
                {stat.trend && (
                  <div className="text-orange-500 text-xs mt-1 flex items-center gap-1 justify-center">
                    <TrendingUp className="w-3 h-3" />
                    {stat.trend}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features Grid */}
      <section className="py-16 px-4 sm:px-6 max-w-7xl mx-auto">
        <div className="grid md:grid-cols-3 gap-6">
          {[
            {
              icon: Shield,
              title: "Zero-Knowledge Privacy",
              description: "Commitment-based architecture ensures your transactions remain private",
            },
            {
              icon: Bolt,
              title: "10-30 Second Settlement",
              description: "Intent-based solver network provides near-instant bridging",
            },
            {
              icon: Coins,
              title: "Lowest Fees",
              description: "Competitive solver market keeps costs minimal (0.05%)",
            },
          ].map((feature, idx) => (
            <div
              key={idx}
              className="bg-neutral-900 border border-neutral-700 p-6 rounded-lg hover:border-orange-500 transition-all duration-300 hover:-translate-y-1 hover:shadow-lg hover:shadow-orange-500/5 group"
            >
              <feature.icon className="w-10 h-10 text-orange-500 mb-4 transition-transform duration-300 group-hover:scale-110" />
              <h3 className="text-white text-lg font-bold mb-2 tracking-wide">{feature.title}</h3>
              <p className="text-neutral-400 text-sm">{feature.description}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Main Bridge Interface */}
      {activeTab === "bridge" && (
        <section className="py-16 px-4 sm:px-6 max-w-2xl mx-auto">
          <div className="bg-neutral-900 border border-neutral-700 rounded-lg p-6 sm:p-8 shadow-lg shadow-orange-500/5">
            {/* Network Selector */}
            <div className="space-y-4 mb-6">
              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <label className="text-xs text-neutral-400 uppercase tracking-wider mb-2 block">From</label>
                  <button className="w-full bg-neutral-800 border border-neutral-700 rounded p-3 flex items-center justify-between hover:border-orange-500 focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-colors">
                    <div className="flex items-center gap-2">
                      <div className="w-6 h-6 bg-orange-500 rounded-full"></div>
                      <span className="text-white">Mantle L2</span>
                    </div>
                    <ChevronDown className="w-4 h-4 text-neutral-400" />
                  </button>
                </div>

                <button
                  onClick={swapNetworks}
                  className="mt-6 p-3 rounded-full bg-neutral-800 border border-neutral-700 hover:bg-orange-500 hover:border-orange-500 transition-all duration-300 hover:scale-110"
                >
                  <ArrowDownUp className="w-5 h-5 text-white" />
                </button>

                <div className="flex-1">
                  <label className="text-xs text-neutral-400 uppercase tracking-wider mb-2 block">To</label>
                  <button className="w-full bg-neutral-800 border border-neutral-700 rounded p-3 flex items-center justify-between hover:border-orange-500 focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-colors">
                    <div className="flex items-center gap-2">
                      <div className="w-6 h-6 bg-blue-500 rounded-full"></div>
                      <span className="text-white">Ethereum L1</span>
                    </div>
                    <ChevronDown className="w-4 h-4 text-neutral-400" />
                  </button>
                </div>
              </div>
            </div>

            {/* Asset & Amount */}
            <div className="mb-6">
              <label className="text-xs text-neutral-400 uppercase tracking-wider mb-2 block">Amount</label>
              <div className="bg-neutral-800 border border-neutral-700 rounded p-4 focus-within:border-orange-500 focus-within:ring-1 focus-within:ring-orange-500 transition-colors">
                <div className="flex items-center justify-between mb-2">
                  <button className="flex items-center gap-2">
                    <div className="w-6 h-6 bg-gradient-to-br from-purple-400 to-blue-500 rounded-full"></div>
                    <span className="text-white font-bold">ETH</span>
                    <ChevronDown className="w-4 h-4 text-neutral-400" />
                  </button>
                  <span className="text-neutral-400 text-sm">Balance: 2.5 ETH</span>
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    value={bridgeAmount}
                    onChange={(e) => setBridgeAmount(e.target.value)}
                    placeholder="0.0"
                    className="flex-1 bg-transparent text-white text-2xl font-mono outline-none"
                  />
                  <button className="text-orange-500 text-sm hover:bg-orange-500/10 px-3 py-1 rounded transition-colors">
                    MAX
                  </button>
                </div>
                <div className="text-neutral-400 text-sm mt-1">
                  ≈ ${bridgeAmount ? (Number.parseFloat(bridgeAmount) * 2500).toFixed(2) : "0.00"} USD
                </div>
              </div>
              <div className="text-neutral-400 text-xs mt-2">
                Fee: 0.05% ({bridgeAmount ? (Number.parseFloat(bridgeAmount) * 0.0005).toFixed(5) : "0.00000"} ETH)
              </div>
            </div>

            {/* Destination */}
            <div className="mb-6">
              <label className="text-xs text-neutral-400 uppercase tracking-wider mb-2 block">
                Destination Address
              </label>
              <input
                type="text"
                value={destinationAddress}
                onChange={(e) => setDestinationAddress(e.target.value)}
                disabled={useConnectedWallet}
                placeholder="0x..."
                className="w-full bg-neutral-800 border border-neutral-700 rounded p-3 text-white font-mono text-sm outline-none focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-colors disabled:opacity-50"
              />
              <div className="flex items-center gap-2 mt-2">
                <input
                  type="checkbox"
                  id="useWallet"
                  checked={useConnectedWallet}
                  onChange={(e) => setUseConnectedWallet(e.target.checked)}
                  className="w-4 h-4 rounded bg-neutral-800 border-neutral-700 checked:bg-orange-500 checked:border-orange-500 focus:ring-orange-500 focus:ring-offset-0"
                />
                <label htmlFor="useWallet" className="text-neutral-400 text-sm">
                  Use connected wallet
                </label>
              </div>
            </div>

            {/* Summary Box */}
            <div className="bg-neutral-800 border border-orange-500/30 rounded p-4 mb-6 space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-neutral-400">You Send:</span>
                <span className="text-white font-mono">{bridgeAmount || "0"} ETH (Mantle)</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-neutral-400">You Receive:</span>
                <span className="text-white font-mono">
                  ~{bridgeAmount ? (Number.parseFloat(bridgeAmount) * 0.9995).toFixed(5) : "0"} ETH (Ethereum)
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-neutral-400">Estimated Time:</span>
                <span className="text-white font-mono">~15 seconds</span>
              </div>
            </div>

            {/* Bridge Button */}
            <Button
              onClick={handleBridgeNow}
              disabled={!bridgeAmount || Number.parseFloat(bridgeAmount) <= 0}
              className="w-full bg-orange-500 hover:bg-orange-600 text-white py-6 text-lg font-bold shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-102 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
            >
              Bridge Now
            </Button>
          </div>
        </section>
      )}

      {/* Activity Table */}
      {activeTab === "activity" && (
        <section className="py-16 px-4 sm:px-6 max-w-7xl mx-auto">
          <div className="mb-6 flex flex-col sm:flex-row gap-4">
            <div className="flex-1 relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-neutral-400" />
              <input
                type="text"
                placeholder="Search by hash or address..."
                className="w-full bg-neutral-900 border border-neutral-700 rounded pl-10 pr-4 py-3 text-white text-sm outline-none focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-colors"
              />
            </div>
            <Button
              variant="outline"
              className="border-neutral-700 text-neutral-400 hover:text-white hover:bg-neutral-800 bg-transparent"
            >
              <Filter className="w-4 h-4 mr-2" />
              Filter
            </Button>
          </div>

          <div className="bg-neutral-900 border border-neutral-700 rounded-lg overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-neutral-800 border-b border-neutral-700">
                  <tr>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Status</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Time</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Route</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Amount</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Fee</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Hash</th>
                    <th className="text-left p-4 text-xs text-neutral-400 uppercase tracking-wider">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    {
                      status: "pending",
                      time: "2m ago",
                      from: "Mantle",
                      to: "Ethereum",
                      amount: "1.5",
                      usd: "2,250",
                      fee: "0.00075",
                      hash: "0x7f3a...9b2c",
                    },
                    {
                      status: "success",
                      time: "15m ago",
                      from: "Ethereum",
                      to: "Mantle",
                      amount: "0.8",
                      usd: "1,200",
                      fee: "0.00040",
                      hash: "0x2b4e...8c1a",
                    },
                    {
                      status: "success",
                      time: "1h ago",
                      from: "Mantle",
                      to: "Ethereum",
                      amount: "2.3",
                      usd: "3,450",
                      fee: "0.00115",
                      hash: "0x9a1c...3f7d",
                    },
                    {
                      status: "pending",
                      time: "2h ago",
                      from: "Ethereum",
                      to: "Mantle",
                      amount: "1.2",
                      usd: "1,800",
                      fee: "0.00060",
                      hash: "0x4d8f...6e2b",
                    },
                    {
                      status: "success",
                      time: "3h ago",
                      from: "Mantle",
                      to: "Ethereum",
                      amount: "0.5",
                      usd: "750",
                      fee: "0.00025",
                      hash: "0x1e7b...4a9c",
                    },
                  ].map((tx, idx) => (
                    <tr
                      key={idx}
                      className="border-b border-neutral-700 hover:bg-neutral-800 hover:border-l-4 hover:border-l-orange-500 transition-all"
                    >
                      <td className="p-4">
                        {tx.status === "pending" && (
                          <div className="flex items-center gap-2">
                            <Loader2 className="w-4 h-4 text-orange-500 animate-spin" />
                            <span className="text-orange-500 text-sm">Pending</span>
                          </div>
                        )}
                        {tx.status === "success" && (
                          <div className="flex items-center gap-2">
                            <CheckCircle2 className="w-4 h-4 text-white" />
                            <span className="text-white text-sm">Success</span>
                          </div>
                        )}
                      </td>
                      <td className="p-4 text-neutral-400 text-sm font-mono">{tx.time}</td>
                      <td className="p-4 text-white text-sm">
                        {tx.from} → {tx.to}
                      </td>
                      <td className="p-4">
                        <div className="text-white font-mono text-sm">{tx.amount} ETH</div>
                        <div className="text-neutral-400 text-xs">${tx.usd}</div>
                      </td>
                      <td className="p-4 text-white font-mono text-sm">{tx.fee} ETH</td>
                      <td className="p-4">
                        <button
                          onClick={() => copyToClipboard(tx.hash, `tx-${idx}`)}
                          className="flex items-center gap-2 text-white font-mono text-sm hover:text-orange-500 transition-colors"
                        >
                          {tx.hash}
                          {copiedAddress === `tx-${idx}` ? (
                            <Check className="w-3 h-3 text-orange-500" />
                          ) : (
                            <Copy className="w-3 h-3" />
                          )}
                        </button>
                      </td>
                      <td className="p-4">
                        <button className="text-orange-500 hover:text-orange-400 text-sm transition-colors">
                          Details
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            <div className="border-t border-neutral-700 p-4 flex items-center justify-between">
              <span className="text-neutral-400 text-sm">Showing 1-5 of 45,231</span>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="border-neutral-700 text-neutral-400 hover:border-orange-500 hover:text-orange-500 bg-transparent"
                >
                  Previous
                </Button>
                <Button variant="outline" size="sm" className="border-orange-500 text-orange-500 bg-transparent">
                  1
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="border-neutral-700 text-neutral-400 hover:border-orange-500 hover:text-orange-500 bg-transparent"
                >
                  2
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="border-neutral-700 text-neutral-400 hover:border-orange-500 hover:text-orange-500 bg-transparent"
                >
                  3
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="border-neutral-700 text-neutral-400 hover:border-orange-500 hover:text-orange-500 bg-transparent"
                >
                  Next
                </Button>
              </div>
            </div>
          </div>
        </section>
      )}

      {/* Stats Dashboard */}
      {activeTab === "stats" && (
        <section className="py-16 px-4 sm:px-6 max-w-7xl mx-auto">
          {/* Top Row - Metrics */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
            {[
              { label: "Total Volume", value: "$12.5M", change: "+15.3%", positive: true },
              { label: "Total Transactions", value: "45,231", change: "+8.2%", positive: true },
              { label: "Avg Bridge Time", value: "18s", change: null, positive: null },
              { label: "Success Rate", value: "98.7%", change: null, positive: null },
            ].map((metric, idx) => (
              <div
                key={idx}
                className="bg-neutral-900 border border-neutral-700 rounded-lg p-6 hover:border-orange-500/50 transition-all duration-300"
              >
                <div className="text-neutral-400 text-xs uppercase tracking-wider mb-2">{metric.label}</div>
                <div className="text-white text-3xl font-bold font-mono mb-1">{metric.value}</div>
                {metric.change && (
                  <div
                    className={`text-sm flex items-center gap-1 ${metric.positive ? "text-orange-500" : "text-red-500"}`}
                  >
                    <TrendingUp className="w-4 h-4" />
                    {metric.change}
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Bottom Row - Charts */}
          <div className="grid lg:grid-cols-2 gap-6">
            {/* Volume Chart */}
            <div className="bg-neutral-900 border border-neutral-700 rounded-lg p-6">
              <h3 className="text-white text-lg font-bold mb-4 tracking-wide">Volume Over Time</h3>
              <div className="h-64 flex items-end gap-2">
                {[40, 65, 45, 80, 70, 90, 85, 95, 75, 100, 88, 92].map((height, idx) => (
                  <div key={idx} className="flex-1 bg-neutral-800 relative group">
                    <div
                      className="absolute bottom-0 w-full bg-orange-500 transition-all duration-500 group-hover:bg-orange-400"
                      style={{ height: `${height}%` }}
                    />
                  </div>
                ))}
              </div>
              <div className="flex justify-between mt-4 text-xs text-neutral-500 font-mono">
                <span>Jan</span>
                <span>Dec</span>
              </div>
            </div>

            {/* Bridge Time Distribution */}
            <div className="bg-neutral-900 border border-neutral-700 rounded-lg p-6">
              <h3 className="text-white text-lg font-bold mb-4 tracking-wide">Bridge Time Distribution</h3>
              <div className="h-64 flex items-end gap-4">
                {[
                  { label: "< 10s", value: 25 },
                  { label: "10-20s", value: 60 },
                  { label: "20-30s", value: 85 },
                  { label: "30-40s", value: 40 },
                  { label: "> 40s", value: 15 },
                ].map((bar, idx) => (
                  <div key={idx} className="flex-1 flex flex-col items-center gap-2">
                    <div className="w-full bg-neutral-800 relative group flex-1 flex items-end">
                      <div
                        className="w-full bg-orange-500 transition-all duration-500 group-hover:bg-orange-400"
                        style={{ height: `${bar.value}%` }}
                      />
                    </div>
                    <span className="text-xs text-neutral-500 font-mono">{bar.label}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* Progress Modal */}
      {showProgressModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={() => setShowProgressModal(false)} />
          <div className="relative bg-neutral-900 border border-orange-500 rounded-lg p-6 sm:p-8 max-w-md w-full shadow-2xl shadow-orange-500/20">
            <h3 className="text-white text-xl font-bold mb-6 tracking-wide">Bridge in Progress</h3>

            {/* Timeline */}
            <div className="space-y-6">
              {[
                { label: "Submitting Intent...", icon: Clock, step: 0 },
                { label: "Waiting for Solver...", icon: Search, step: 1 },
                { label: "Solver Filled", icon: Zap, step: 2 },
                { label: "Auto-Claiming", icon: Loader2, step: 3 },
                { label: "Complete!", icon: CheckCircle2, step: 4 },
              ].map((item, idx) => {
                const isActive = progressStep === item.step
                const isCompleted = progressStep > item.step
                const isPending = progressStep < item.step

                return (
                  <div key={idx} className="flex items-center gap-4">
                    <div
                      className={`flex items-center justify-center w-10 h-10 rounded-full border-2 transition-all ${
                        isCompleted
                          ? "border-orange-500 bg-orange-500/20"
                          : isActive
                            ? "border-orange-500 bg-orange-500/10"
                            : "border-neutral-700 bg-neutral-800"
                      }`}
                    >
                      <item.icon
                        className={`w-5 h-5 ${
                          isCompleted || isActive ? "text-orange-500" : "text-neutral-500"
                        } ${isActive && item.step === 1 ? "animate-spin" : ""}`}
                      />
                    </div>
                    <div className="flex-1">
                      <div className={`text-sm ${isCompleted || isActive ? "text-white" : "text-neutral-500"}`}>
                        {item.label}
                      </div>
                      {isActive && item.step === 1 && (
                        <div className="mt-2 bg-neutral-800 rounded-full h-2 overflow-hidden">
                          <div
                            className="bg-orange-500 h-full rounded-full transition-all duration-1000"
                            style={{ width: "45%" }}
                          />
                        </div>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>

            {/* Transaction Hash */}
            <div className="mt-6 p-4 bg-neutral-800 border border-neutral-700 rounded">
              <div className="text-xs text-neutral-400 uppercase tracking-wider mb-2">Transaction Hash</div>
              <button
                onClick={() =>
                  copyToClipboard("0x7f3a2b4e9c1d8f6a3e5b7c9d2f4a6b8c1e3d5f7a9b2c4e6f8a1c3e5d7f9b1c3e", "hash")
                }
                className="flex items-center justify-between w-full group"
              >
                <span className="text-white font-mono text-sm truncate">0x7f3a...9b1c3e</span>
                {copiedHash ? (
                  <Check className="w-4 h-4 text-orange-500 flex-shrink-0 ml-2" />
                ) : (
                  <Copy className="w-4 h-4 text-neutral-400 group-hover:text-orange-500 transition-colors flex-shrink-0 ml-2" />
                )}
              </button>
            </div>

            {progressStep === 4 && (
              <Button
                onClick={() => setShowProgressModal(false)}
                className="w-full mt-6 bg-orange-500 hover:bg-orange-600 text-white transition-all duration-300"
              >
                Done
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Footer */}
      <footer className="border-t border-neutral-700 bg-neutral-900 py-8 px-4 sm:px-6 mt-16">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="text-neutral-500 text-sm">© 2025 Shadow Swap. Privacy-enhanced bridging protocol.</div>
          <div className="flex items-center gap-6 text-sm">
            <a href="#" className="text-neutral-400 hover:text-orange-500 transition-colors">
              Docs
            </a>
            <a href="#" className="text-neutral-400 hover:text-orange-500 transition-colors">
              Twitter
            </a>
            <a href="#" className="text-neutral-400 hover:text-orange-500 transition-colors">
              Discord
            </a>
            <a href="#" className="text-neutral-400 hover:text-orange-500 transition-colors">
              GitHub
            </a>
          </div>
        </div>
      </footer>
    </div>
  )
}
