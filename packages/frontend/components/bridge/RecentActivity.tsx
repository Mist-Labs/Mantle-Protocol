"use client"

import { useMemo } from "react";
import { motion } from "framer-motion";
import Link from "next/link";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  CheckCircle2,
  Clock,
  XCircle,
  ArrowRight,
  Loader2,
  ExternalLink,
  AlertCircle,
} from "lucide-react";
import { useBridgeIntents, formatChainName } from "@/hooks/useBridgeIntents";
import { getTxUrl } from "@/lib/contracts";
import type { ChainType } from "@/lib/tokens";

export default function RecentActivity() {
  // Fetch ALL transactions from backend with real-time updates (not filtered by user)
  const { intents, isLoading } = useBridgeIntents({
    limit: 5,
    // Don't pass userAddress - show ALL transactions
  });

  // Get recent transactions (already limited to 5 by query)
  const recentTransactions = intents;

  // Count pending transactions
  const pendingCount = useMemo(() => {
    return intents.filter((intent) => {
      const terminalStates = ["completed", "refunded", "failed"];
      return !terminalStates.includes(intent.status);
    }).length;
  }, [intents]);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case "filled":
        return <CheckCircle2 className="h-4 w-4 text-blue-500" />;
      case "committed":
      case "created":
        return <Loader2 className="h-4 w-4 animate-spin text-orange-500" />;
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return <Clock className="h-4 w-4 text-neutral-500" />;
    }
  };

  const getStatusBadge = (status: string) => {
    const variants: Record<string, { class: string; text: string }> = {
      completed: {
        class: "bg-green-500/10 text-green-500 border-green-500/20",
        text: "Completed",
      },
      filled: {
        class: "bg-blue-500/10 text-blue-500 border-blue-500/20",
        text: "Filled",
      },
      committed: {
        class: "bg-purple-500/10 text-purple-500 border-purple-500/20",
        text: "Committed",
      },
      created: {
        class: "bg-orange-500/10 text-orange-500 border-orange-500/20",
        text: "Created",
      },
      failed: {
        class: "bg-red-500/10 text-red-500 border-red-500/20",
        text: "Failed",
      },
    };
    const variant = variants[status] || variants.created;
    return (
      <Badge variant="outline" className={`text-xs ${variant.class}`}>
        {variant.text}
      </Badge>
    );
  };

  const formatTimeAgo = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  };

  if (isLoading) {
    return (
      <Card className="border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm">
        <div className="flex items-center justify-center gap-2 text-neutral-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="text-sm">Loading activity...</span>
        </div>
      </Card>
    );
  }

  if (recentTransactions.length === 0) {
    return null; // Don't show if no transactions
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, delay: 0.1 }}
    >
      <Card className="border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm">
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h3 className="text-lg font-bold text-white">Recent Activity</h3>
            <p className="text-sm text-neutral-400">
              {pendingCount > 0
                ? `${pendingCount} pending transaction${pendingCount > 1 ? "s" : ""}`
                : "All transactions completed"}
            </p>
          </div>
          <Link href="/activity">
            <Button
              size="sm"
              variant="outline"
              className="border-neutral-700 bg-neutral-800 text-xs hover:bg-neutral-700"
            >
              View All
              <ExternalLink className="ml-1 h-3 w-3" />
            </Button>
          </Link>
        </div>

        {/* Pending Alert */}
        {pendingCount > 0 && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="mb-4 flex items-start gap-2 rounded-lg border border-orange-500/20 bg-orange-500/10 p-3"
          >
            <AlertCircle className="h-4 w-4 flex-shrink-0 text-orange-500" />
            <div className="flex-1">
              <p className="text-xs font-medium text-orange-400">
                {pendingCount} transaction{pendingCount > 1 ? "s" : ""}{" "}
                in progress
              </p>
              <p className="mt-0.5 text-xs text-neutral-400">
                Auto-updating every 5 seconds
              </p>
            </div>
          </motion.div>
        )}

        {/* Transaction List */}
        <div className="space-y-3">
          {recentTransactions.filter(intent => intent.intent_id).map((intent, index) => {
            // Helper to get token symbol from address
            const getTokenSymbol = (token: string | undefined) => {
              if (!token) return "Unknown";
              if (token.startsWith("0x")) {
                const lowerToken = token.toLowerCase();

                // USDC addresses (Ethereum Sepolia & Mantle Sepolia)
                if (lowerToken === "0x28650373758d75a8ff0b22587f111e47bac34e21" ||
                    lowerToken === "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238" ||
                    lowerToken === "0xa4b184006b59861f80521649b14e4e8a72499a23") {
                  return "USDC"
                }

                // USDT addresses (Ethereum Sepolia & Mantle Sepolia)
                if (lowerToken === "0x89f4f0e13997ca27ceb963dee291c607e4e59923" ||
                    lowerToken === "0xb0ee6ef7788e9122fc4aae327ed4fef56c7da891" ||
                    lowerToken === "0x3e77a87143e8d1da601094d9d83006a982b194d3" ||
                    lowerToken === "0xf417f5a458ec102b90352f697d6e2ac3a3d2851f") {
                  return "USDT"
                }

                // WETH addresses
                if (lowerToken === "0x50e8da97beeb8064714de45ce1f250879f3bd5b5" ||
                    lowerToken === "0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111") {
                  return "WETH"
                }

                // MNT addresses
                if (lowerToken === "0x65e37b558f64e2be5768db46df22f93d85741a9e" ||
                    lowerToken === "0x44fce297e4d6c5a50d28fb26a58202e4d49a13e7") {
                  return "MNT"
                }

                // ETH (native)
                if (lowerToken === "0x0000000000000000000000000000000000000000") {
                  return "ETH"
                }

                return `${token.slice(0, 6)}...${token.slice(-4)}`
              }
              return token
            }

            // Format amount from wei to human-readable
            const formatAmount = (amount: string | undefined) => {
              if (!amount) return "0";
              try {
                const value = BigInt(amount);
                const decimals = 6; // USDC/USDT
                const quotient = value / BigInt(10 ** decimals);
                const remainder = value % BigInt(10 ** decimals);
                if (remainder === BigInt(0)) return quotient.toString();
                const remainderStr = remainder.toString().padStart(decimals, "0");
                const trimmed = remainderStr.slice(0, 6).replace(/0+$/, "");
                return trimmed ? `${quotient}.${trimmed}` : quotient.toString();
              } catch {
                return amount;
              }
            }

            // Format time ago
            const formatTimeAgo = (dateString: string | undefined) => {
              if (!dateString) return "Unknown";
              try {
                const seconds = Math.floor((Date.now() - new Date(dateString).getTime()) / 1000);
                if (seconds < 60) return `${seconds}s ago`;
                const minutes = Math.floor(seconds / 60);
                if (minutes < 60) return `${minutes}m ago`;
                const hours = Math.floor(minutes / 60);
                if (hours < 24) return `${hours}h ago`;
                const days = Math.floor(hours / 24);
                return `${days}d ago`;
              } catch {
                return "Unknown";
              }
            }

            return (
              <motion.div
                key={intent.intent_id}
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05 }}
                className="group relative overflow-hidden rounded-lg border border-neutral-700/50 bg-neutral-800/30 p-3 transition-all hover:border-orange-500/30 hover:bg-neutral-800/50"
              >
                {/* Status Indicator Bar */}
                <div
                  className={`absolute left-0 top-0 h-full w-1 ${
                    intent.status === "completed"
                      ? "bg-green-500"
                      : intent.status === "failed"
                        ? "bg-red-500"
                        : "bg-orange-500"
                  }`}
                />

                <div className="flex items-center justify-between">
                  {/* Left Side */}
                  <div className="flex items-center gap-3">
                    {getStatusIcon(intent.status)}
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-white">
                          {formatAmount(intent.amount)} {getTokenSymbol(intent.source_token)}
                        </span>
                        {getStatusBadge(intent.status)}
                      </div>
                      <div className="mt-0.5 flex items-center gap-1.5 text-xs text-neutral-500">
                        <span>{formatChainName(intent.source_chain)}</span>
                        <ArrowRight className="h-3 w-3" />
                        <span>{formatChainName(intent.dest_chain)}</span>
                        <span>•</span>
                        <span>{formatTimeAgo(intent.created_at)}</span>
                      </div>
                    </div>
                  </div>

                  {/* Right Side */}
                  <div className="flex items-center gap-2">
                    {intent.source_complete_txid && (
                      <a
                        href={getTxUrl(intent.source_chain as ChainType, intent.source_complete_txid as `0x${string}`)}
                        target="_blank"
                        rel="noopener noreferrer"
                        onClick={(e) => e.stopPropagation()}
                        className="opacity-0 transition-opacity group-hover:opacity-100"
                      >
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-7 px-2 text-neutral-400 hover:text-orange-500"
                        >
                          <ExternalLink className="h-3.5 w-3.5" />
                        </Button>
                      </a>
                    )}
                  </div>
                </div>

                {/* Intent ID (truncated) */}
                <div className="mt-2 flex items-center gap-1 text-xs text-neutral-600">
                  <span className="font-mono">
                    {intent.intent_id.slice(0, 6)}...{intent.intent_id.slice(-4)}
                  </span>
                </div>
              </motion.div>
            )
          })}
        </div>

        {/* Footer */}
        {recentTransactions.length >= 5 && (
          <div className="mt-4 text-center">
            <Link href="/activity">
              <Button
                variant="ghost"
                size="sm"
                className="text-xs text-neutral-400 hover:text-orange-500"
              >
                View all transactions
              </Button>
            </Link>
          </div>
        )}
      </Card>
    </motion.div>
  );
}
