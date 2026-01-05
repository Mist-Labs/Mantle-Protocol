"use client"

import { useState, useMemo } from "react"
import { motion } from "framer-motion"
import Navigation from "@/components/shared/Navigation"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import {
  Search,
  Filter,
  ExternalLink,
  Copy,
  Check,
  CheckCircle2,
  Clock,
  XCircle,
  ArrowRight,
  Loader2,
  RefreshCw,
  AlertCircle,
} from "lucide-react"
import { useBridgeIntents, formatTimeAgo, formatChainName, formatAmount } from "@/hooks/useBridgeIntents"
import type { IntentStatusResponse, IntentStatus } from "@/lib/api"
import type { ChainType } from "@/lib/tokens"

export default function ActivityPage() {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [networkFilter, setNetworkFilter] = useState<string>("all")
  const [selectedTx, setSelectedTx] = useState<IntentStatusResponse | null>(null)
  const [copiedField, setCopiedField] = useState<string | null>(null)

  // Fetch bridge intents with filters
  const filterStatus = statusFilter !== "all" ? (statusFilter as IntentStatus) : undefined
  const filterChain = networkFilter !== "all" ? (networkFilter as ChainType) : undefined

  const { intents, count, isLoading, error, refetch } = useBridgeIntents({
    status: filterStatus,
    chain: filterChain,
    limit: 50,
  })

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text)
    setCopiedField(field)
    setTimeout(() => setCopiedField(null), 2000)
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />
      case "filled":
        return <CheckCircle2 className="h-4 w-4 text-blue-500" />
      case "committed":
      case "created":
        return <Loader2 className="h-4 w-4 animate-spin text-orange-500" />
      case "refunded":
        return <ArrowRight className="h-4 w-4 text-yellow-500" />
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />
      default:
        return <Clock className="h-4 w-4 text-neutral-500" />
    }
  }

  const getStatusBadge = (status: string) => {
    const variants: Record<string, { class: string; text: string }> = {
      completed: { class: "bg-green-500/10 text-green-500 border-green-500/20", text: "Completed" },
      filled: { class: "bg-blue-500/10 text-blue-500 border-blue-500/20", text: "Filled" },
      committed: { class: "bg-purple-500/10 text-purple-500 border-purple-500/20", text: "Committed" },
      created: { class: "bg-orange-500/10 text-orange-500 border-orange-500/20", text: "Created" },
      refunded: { class: "bg-yellow-500/10 text-yellow-500 border-yellow-500/20", text: "Refunded" },
      failed: { class: "bg-red-500/10 text-red-500 border-red-500/20", text: "Failed" },
    }
    const variant = variants[status] || variants.created
    return (
      <Badge variant="outline" className={variant.class}>
        {variant.text}
      </Badge>
    )
  }

  // Client-side filtering for search queries
  const filteredTransactions = useMemo(() => {
    return intents.filter((intent) => {
      const matchesSearch =
        searchQuery === "" ||
        intent.intent_id.toLowerCase().includes(searchQuery.toLowerCase()) ||
        intent.source_chain.toLowerCase().includes(searchQuery.toLowerCase()) ||
        intent.dest_chain.toLowerCase().includes(searchQuery.toLowerCase()) ||
        intent.source_token.toLowerCase().includes(searchQuery.toLowerCase()) ||
        intent.commitment.toLowerCase().includes(searchQuery.toLowerCase())

      return matchesSearch
    })
  }, [intents, searchQuery])

  return (
    <div className="min-h-screen bg-black">
      <Navigation />

      {/* Main Content */}
      <main className="px-4 pb-12 pt-24 sm:px-6">
        <div className="mx-auto max-w-7xl">
          {/* Header */}
          <div className="mb-8 flex items-center justify-between">
            <div>
              <h1 className="mb-2 text-3xl font-bold text-white sm:text-4xl">Transaction Activity</h1>
              <p className="text-neutral-400">View and track your cross-chain bridge transactions</p>
            </div>
            <Button
              onClick={refetch}
              disabled={isLoading}
              variant="outline"
              className="border-neutral-700 bg-neutral-800 hover:bg-neutral-700"
            >
              <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          </div>

          {/* Error State */}
          {error && (
            <div className="mb-6 flex items-center gap-3 rounded-lg border border-red-500/20 bg-red-500/10 p-4 text-red-500">
              <AlertCircle className="h-5 w-5" />
              <div>
                <p className="font-medium">Error loading transactions</p>
                <p className="text-sm text-red-400">{error}</p>
              </div>
            </div>
          )}

          {/* Filters */}
          <div className="mb-6 rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
              {/* Search */}
              <div className="md:col-span-2">
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform text-neutral-500" />
                  <Input
                    placeholder="Search by intent ID, chain, token, commitment..."
                    className="border-neutral-700 bg-neutral-800 pl-10 text-white"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    disabled={isLoading}
                  />
                </div>
              </div>

              {/* Status Filter */}
              <div>
                <Select value={statusFilter} onValueChange={setStatusFilter} disabled={isLoading}>
                  <SelectTrigger className="border-neutral-700 bg-neutral-800 text-white">
                    <SelectValue placeholder="All Statuses" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Statuses</SelectItem>
                    <SelectItem value="created">Created</SelectItem>
                    <SelectItem value="committed">Committed</SelectItem>
                    <SelectItem value="filled">Filled</SelectItem>
                    <SelectItem value="completed">Completed</SelectItem>
                    <SelectItem value="refunded">Refunded</SelectItem>
                    <SelectItem value="failed">Failed</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Network Filter */}
              <div>
                <Select value={networkFilter} onValueChange={setNetworkFilter} disabled={isLoading}>
                  <SelectTrigger className="border-neutral-700 bg-neutral-800 text-white">
                    <SelectValue placeholder="All Networks" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Networks</SelectItem>
                    <SelectItem value="mantle">Mantle</SelectItem>
                    <SelectItem value="ethereum">Ethereum</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          {/* Transactions Table */}
          <div className="overflow-hidden rounded-lg border border-neutral-800 bg-neutral-900">
            <Table>
              <TableHeader>
                <TableRow className="border-neutral-800 hover:bg-neutral-800/50">
                  <TableHead className="text-neutral-400">Status</TableHead>
                  <TableHead className="text-neutral-400">Time</TableHead>
                  <TableHead className="text-neutral-400">Route</TableHead>
                  <TableHead className="text-neutral-400">Amount</TableHead>
                  <TableHead className="text-neutral-400">Fee</TableHead>
                  <TableHead className="text-neutral-400">Hash</TableHead>
                  <TableHead className="text-right text-neutral-400">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading ? (
                  <TableRow>
                    <TableCell colSpan={7} className="py-12 text-center">
                      <div className="flex items-center justify-center gap-2 text-neutral-500">
                        <Loader2 className="h-5 w-5 animate-spin" />
                        <span>Loading transactions...</span>
                      </div>
                    </TableCell>
                  </TableRow>
                ) : filteredTransactions.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={7} className="py-12 text-center text-neutral-500">
                      No transactions found
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredTransactions.map((intent) => (
                    <TableRow
                      key={intent.intent_id}
                      className="cursor-pointer border-neutral-800 hover:bg-neutral-800/30"
                      onClick={() => setSelectedTx(intent)}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2">
                          {getStatusIcon(intent.status)}
                          {getStatusBadge(intent.status)}
                        </div>
                      </TableCell>
                      <TableCell className="text-neutral-300">{formatTimeAgo(intent.created_at)}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span className="text-neutral-300">{formatChainName(intent.source_chain)}</span>
                          <ArrowRight className="h-3 w-3 text-orange-500" />
                          <span className="text-neutral-300">{formatChainName(intent.dest_chain)}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div>
                          <div className="font-medium text-white">
                            {formatAmount(intent.amount)} {intent.source_token}
                          </div>
                          {intent.has_privacy && (
                            <div className="text-xs text-purple-400">🔒 Private</div>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-neutral-300">~0.15%</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-sm text-neutral-300">
                            {intent.intent_id.slice(0, 6)}...{intent.intent_id.slice(-4)}
                          </span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              copyToClipboard(intent.intent_id, intent.intent_id)
                            }}
                            className="text-neutral-500 transition-colors hover:text-orange-500"
                          >
                            {copiedField === intent.intent_id ? (
                              <Check className="h-4 w-4 text-green-500" />
                            ) : (
                              <Copy className="h-4 w-4" />
                            )}
                          </button>
                        </div>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          className="border-neutral-700 bg-neutral-800 hover:border-orange-500/50 hover:bg-neutral-700"
                          onClick={(e) => {
                            e.stopPropagation()
                            setSelectedTx(intent)
                          }}
                        >
                          View Details
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          {/* Pagination */}
          <div className="mt-6 flex items-center justify-between">
            <p className="text-sm text-neutral-500">
              Showing {filteredTransactions.length} of {count} transactions
            </p>
            {count > 50 && (
              <div className="flex gap-2">
                <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800" disabled>
                  Previous
                </Button>
                <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800" disabled>
                  Next
                </Button>
              </div>
            )}
          </div>
        </div>
      </main>

      {/* Transaction Detail Modal */}
      <Dialog open={!!selectedTx} onOpenChange={() => setSelectedTx(null)}>
        <DialogContent className="max-w-2xl border border-neutral-800 bg-neutral-900 text-white">
          <DialogHeader>
            <DialogTitle className="text-2xl font-bold">Transaction Details</DialogTitle>
            <DialogDescription className="text-neutral-400">
              View complete information about this bridge transaction
            </DialogDescription>
          </DialogHeader>

          {selectedTx && (
            <div className="space-y-6">
              {/* Status */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Status</span>
                <div className="flex items-center gap-2">
                  {getStatusIcon(selectedTx.status)}
                  {getStatusBadge(selectedTx.status)}
                </div>
              </div>

              {/* Route */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Route</span>
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="border-neutral-700">
                    {formatChainName(selectedTx.source_chain)}
                  </Badge>
                  <ArrowRight className="h-4 w-4 text-orange-500" />
                  <Badge variant="outline" className="border-neutral-700">
                    {formatChainName(selectedTx.dest_chain)}
                  </Badge>
                </div>
              </div>

              {/* Amount */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Amount</span>
                <div className="text-right">
                  <div className="font-medium text-white">
                    {formatAmount(selectedTx.amount)} {selectedTx.source_token}
                  </div>
                  <div className="text-sm text-neutral-500">→ {selectedTx.dest_token}</div>
                </div>
              </div>

              {/* Privacy */}
              {selectedTx.has_privacy && (
                <div className="flex items-center justify-between">
                  <span className="text-neutral-400">Privacy</span>
                  <Badge variant="outline" className="border-purple-500/20 bg-purple-500/10 text-purple-400">
                    🔒 Privacy-Enhanced
                  </Badge>
                </div>
              )}

              {/* Deadline */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Deadline</span>
                <span className="text-white">
                  {new Date(selectedTx.deadline * 1000).toLocaleString()}
                </span>
              </div>

              {/* Time */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Created</span>
                <span className="text-white">
                  {new Date(selectedTx.created_at).toLocaleString()}
                </span>
              </div>

              {/* Intent ID */}
              <div>
                <div className="mb-2 text-neutral-400">Intent ID</div>
                <div className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 p-3">
                  <span className="flex-1 truncate font-mono text-sm text-white">{selectedTx.intent_id}</span>
                  <button
                    onClick={() => copyToClipboard(selectedTx.intent_id, "modal-intent")}
                    className="text-neutral-500 transition-colors hover:text-orange-500"
                  >
                    {copiedField === "modal-intent" ? (
                      <Check className="h-4 w-4 text-green-500" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </button>
                </div>
              </div>

              {/* Commitment */}
              <div>
                <div className="mb-2 text-neutral-400">Commitment</div>
                <div className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 p-3">
                  <span className="flex-1 truncate font-mono text-sm text-white">{selectedTx.commitment}</span>
                  <button
                    onClick={() => copyToClipboard(selectedTx.commitment, "modal-commitment")}
                    className="text-neutral-500 transition-colors hover:text-orange-500"
                  >
                    {copiedField === "modal-commitment" ? (
                      <Check className="h-4 w-4 text-green-500" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </button>
                </div>
              </div>

              {/* Transaction Hashes */}
              {selectedTx.dest_fill_txid && (
                <div>
                  <div className="mb-2 text-neutral-400">Fill Transaction</div>
                  <div className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 p-3">
                    <span className="flex-1 truncate font-mono text-sm text-white">{selectedTx.dest_fill_txid}</span>
                    <button
                      onClick={() => copyToClipboard(selectedTx.dest_fill_txid!, "modal-fill")}
                      className="text-neutral-500 transition-colors hover:text-orange-500"
                    >
                      {copiedField === "modal-fill" ? (
                        <Check className="h-4 w-4 text-green-500" />
                      ) : (
                        <Copy className="h-4 w-4" />
                      )}
                    </button>
                    <a
                      href={`${
                        selectedTx.dest_chain === "ethereum"
                          ? process.env.NEXT_PUBLIC_ETHEREUM_EXPLORER
                          : process.env.NEXT_PUBLIC_MANTLE_EXPLORER
                      }/tx/${selectedTx.dest_fill_txid}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-neutral-500 transition-colors hover:text-orange-500"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </div>
                </div>
              )}

              {selectedTx.source_complete_txid && (
                <div>
                  <div className="mb-2 text-neutral-400">Complete Transaction</div>
                  <div className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 p-3">
                    <span className="flex-1 truncate font-mono text-sm text-white">
                      {selectedTx.source_complete_txid}
                    </span>
                    <button
                      onClick={() => copyToClipboard(selectedTx.source_complete_txid!, "modal-complete")}
                      className="text-neutral-500 transition-colors hover:text-orange-500"
                    >
                      {copiedField === "modal-complete" ? (
                        <Check className="h-4 w-4 text-green-500" />
                      ) : (
                        <Copy className="h-4 w-4" />
                      )}
                    </button>
                    <a
                      href={`${
                        selectedTx.source_chain === "ethereum"
                          ? process.env.NEXT_PUBLIC_ETHEREUM_EXPLORER
                          : process.env.NEXT_PUBLIC_MANTLE_EXPLORER
                      }/tx/${selectedTx.source_complete_txid}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-neutral-500 transition-colors hover:text-orange-500"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </div>
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
