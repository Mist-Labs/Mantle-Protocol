"use client"

import { useState } from "react"
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
} from "lucide-react"

export default function ActivityPage() {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState("all")
  const [networkFilter, setNetworkFilter] = useState("all")
  const [selectedTx, setSelectedTx] = useState<any>(null)
  const [copiedField, setCopiedField] = useState<string | null>(null)

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text)
    setCopiedField(field)
    setTimeout(() => setCopiedField(null), 2000)
  }

  // Mock transaction data
  const transactions = [
    {
      id: "1",
      hash: "0x1234...5678",
      fullHash: "0x1234567890abcdef1234567890abcdef12345678",
      from: "Mantle L2",
      to: "Ethereum L1",
      amount: "1.5 ETH",
      usdValue: "$3,750",
      fee: "0.00075 ETH",
      status: "completed",
      time: "2 hours ago",
      timestamp: "2025-12-14 22:30:15",
    },
    {
      id: "2",
      hash: "0xabcd...ef12",
      fullHash: "0xabcdef1234567890abcdef1234567890abcdef12",
      from: "Ethereum L1",
      to: "Mantle L2",
      amount: "0.8 ETH",
      usdValue: "$2,000",
      fee: "0.0004 ETH",
      status: "completed",
      time: "5 hours ago",
      timestamp: "2025-12-14 19:15:42",
    },
    {
      id: "3",
      hash: "0x9876...4321",
      fullHash: "0x9876543210fedcba9876543210fedcba98765432",
      from: "Mantle L2",
      to: "Ethereum L1",
      amount: "2.3 ETH",
      usdValue: "$5,750",
      fee: "0.00115 ETH",
      status: "pending",
      time: "10 minutes ago",
      timestamp: "2025-12-15 00:15:30",
    },
    {
      id: "4",
      hash: "0xfedc...ba98",
      fullHash: "0xfedcba9876543210fedcba9876543210fedcba98",
      from: "Ethereum L1",
      to: "Mantle L2",
      amount: "0.5 ETH",
      usdValue: "$1,250",
      fee: "0.00025 ETH",
      status: "failed",
      time: "1 day ago",
      timestamp: "2025-12-13 15:45:22",
    },
  ]

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />
      case "pending":
        return <Loader2 className="h-4 w-4 animate-spin text-orange-500" />
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />
      default:
        return <Clock className="h-4 w-4 text-neutral-500" />
    }
  }

  const getStatusBadge = (status: string) => {
    const variants: Record<string, { class: string; text: string }> = {
      completed: { class: "bg-green-500/10 text-green-500 border-green-500/20", text: "Completed" },
      pending: { class: "bg-orange-500/10 text-orange-500 border-orange-500/20", text: "Pending" },
      failed: { class: "bg-red-500/10 text-red-500 border-red-500/20", text: "Failed" },
    }
    const variant = variants[status] || variants.completed
    return (
      <Badge variant="outline" className={variant.class}>
        {variant.text}
      </Badge>
    )
  }

  const filteredTransactions = transactions.filter((tx) => {
    const matchesSearch =
      tx.hash.toLowerCase().includes(searchQuery.toLowerCase()) ||
      tx.from.toLowerCase().includes(searchQuery.toLowerCase()) ||
      tx.to.toLowerCase().includes(searchQuery.toLowerCase())

    const matchesStatus = statusFilter === "all" || tx.status === statusFilter
    const matchesNetwork =
      networkFilter === "all" ||
      tx.from.toLowerCase().includes(networkFilter.toLowerCase()) ||
      tx.to.toLowerCase().includes(networkFilter.toLowerCase())

    return matchesSearch && matchesStatus && matchesNetwork
  })

  return (
    <div className="min-h-screen bg-black">
      <Navigation />

      {/* Main Content */}
      <main className="px-4 pb-12 pt-24 sm:px-6">
        <div className="mx-auto max-w-7xl">
          {/* Header */}
          <div className="mb-8">
            <h1 className="mb-2 text-3xl font-bold text-white sm:text-4xl">Transaction Activity</h1>
            <p className="text-neutral-400">View and track your cross-chain bridge transactions</p>
          </div>

          {/* Filters */}
          <div className="mb-6 rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
              {/* Search */}
              <div className="md:col-span-2">
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform text-neutral-500" />
                  <Input
                    placeholder="Search by hash, network..."
                    className="border-neutral-700 bg-neutral-800 pl-10 text-white"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                  />
                </div>
              </div>

              {/* Status Filter */}
              <div>
                <Select value={statusFilter} onValueChange={setStatusFilter}>
                  <SelectTrigger className="border-neutral-700 bg-neutral-800 text-white">
                    <SelectValue placeholder="All Statuses" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Statuses</SelectItem>
                    <SelectItem value="completed">Completed</SelectItem>
                    <SelectItem value="pending">Pending</SelectItem>
                    <SelectItem value="failed">Failed</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Network Filter */}
              <div>
                <Select value={networkFilter} onValueChange={setNetworkFilter}>
                  <SelectTrigger className="border-neutral-700 bg-neutral-800 text-white">
                    <SelectValue placeholder="All Networks" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Networks</SelectItem>
                    <SelectItem value="mantle">Mantle L2</SelectItem>
                    <SelectItem value="ethereum">Ethereum L1</SelectItem>
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
                {filteredTransactions.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={7} className="py-12 text-center text-neutral-500">
                      No transactions found
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredTransactions.map((tx) => (
                    <TableRow key={tx.id} className="cursor-pointer border-neutral-800 hover:bg-neutral-800/30">
                      <TableCell>
                        <div className="flex items-center gap-2">
                          {getStatusIcon(tx.status)}
                          {getStatusBadge(tx.status)}
                        </div>
                      </TableCell>
                      <TableCell className="text-neutral-300">{tx.time}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span className="text-neutral-300">{tx.from}</span>
                          <ArrowRight className="h-3 w-3 text-orange-500" />
                          <span className="text-neutral-300">{tx.to}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div>
                          <div className="font-medium text-white">{tx.amount}</div>
                          <div className="text-xs text-neutral-500">{tx.usdValue}</div>
                        </div>
                      </TableCell>
                      <TableCell className="text-neutral-300">{tx.fee}</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-sm text-neutral-300">{tx.hash}</span>
                          <button
                            onClick={() => copyToClipboard(tx.fullHash, tx.id)}
                            className="text-neutral-500 transition-colors hover:text-orange-500"
                          >
                            {copiedField === tx.id ? (
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
                          onClick={() => setSelectedTx(tx)}
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
            <p className="text-sm text-neutral-500">Showing {filteredTransactions.length} transactions</p>
            <div className="flex gap-2">
              <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800" disabled>
                Previous
              </Button>
              <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800" disabled>
                Next
              </Button>
            </div>
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
                    {selectedTx.from}
                  </Badge>
                  <ArrowRight className="h-4 w-4 text-orange-500" />
                  <Badge variant="outline" className="border-neutral-700">
                    {selectedTx.to}
                  </Badge>
                </div>
              </div>

              {/* Amount */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Amount</span>
                <div className="text-right">
                  <div className="font-medium text-white">{selectedTx.amount}</div>
                  <div className="text-sm text-neutral-500">{selectedTx.usdValue}</div>
                </div>
              </div>

              {/* Fee */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Fee</span>
                <span className="text-white">{selectedTx.fee}</span>
              </div>

              {/* Time */}
              <div className="flex items-center justify-between">
                <span className="text-neutral-400">Time</span>
                <span className="text-white">{selectedTx.timestamp}</span>
              </div>

              {/* Transaction Hash */}
              <div>
                <div className="mb-2 text-neutral-400">Transaction Hash</div>
                <div className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 p-3">
                  <span className="flex-1 truncate font-mono text-sm text-white">{selectedTx.fullHash}</span>
                  <button
                    onClick={() => copyToClipboard(selectedTx.fullHash, "modal")}
                    className="text-neutral-500 transition-colors hover:text-orange-500"
                  >
                    {copiedField === "modal" ? (
                      <Check className="h-4 w-4 text-green-500" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </button>
                  <a
                    href="#"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-neutral-500 transition-colors hover:text-orange-500"
                  >
                    <ExternalLink className="h-4 w-4" />
                  </a>
                </div>
              </div>

              {/* Actions */}
              <div className="flex gap-3 pt-4">
                <Button className="flex-1 bg-orange-500 hover:bg-orange-600">
                  <ExternalLink className="mr-2 h-4 w-4" />
                  View on Explorer
                </Button>
                <Button variant="outline" className="border-neutral-700 bg-neutral-800 hover:bg-neutral-700">
                  <Copy className="mr-2 h-4 w-4" />
                  Copy Details
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
