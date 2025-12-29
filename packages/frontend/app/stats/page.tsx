"use client"

import { useState, useMemo } from "react"
import Navigation from "@/components/shared/Navigation"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  TrendingUp,
  TrendingDown,
  Users,
  Clock,
  CheckCircle2,
  DollarSign,
  Activity,
  Zap,
  ArrowUpRight,
  ArrowDownRight,
  Loader2,
  RefreshCw,
  AlertCircle,
} from "lucide-react"
import { useBridgeStats, useHealthCheck, calculateSuccessRate, formatLargeNumber } from "@/hooks/useBridgeStats"
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from "recharts"

export default function StatsPage() {
  const [timePeriod, setTimePeriod] = useState("7d")

  // Fetch real bridge statistics
  const {
    totalIntents,
    pendingIntents,
    filledIntents,
    completedIntents,
    failedIntents,
    refundedIntents,
    ethereumToMantle,
    mantleToEthereum,
    volumeByToken,
    isLoading: statsLoading,
    error: statsError,
    refetch: refetchStats,
  } = useBridgeStats()

  // Fetch health check data
  const {
    status: healthStatus,
    components: healthComponents,
    isLoading: healthLoading,
    error: healthError,
    refetch: refetchHealth,
  } = useHealthCheck()

  // Calculate derived metrics
  const successRate = useMemo(() => {
    return calculateSuccessRate(completedIntents, failedIntents);
  }, [completedIntents, failedIntents])

  // Calculate total volume (approximate sum)
  const totalVolume = useMemo(() => {
    return Object.values(volumeByToken).reduce((acc, val) => {
      const numVal = parseFloat(val) || 0;
      return acc + numVal;
    }, 0);
  }, [volumeByToken])

  // Mock data for charts (will be replaced with real time-series data in future)
  const volumeData = [
    { date: "Dec 8", volume: 1200000 },
    { date: "Dec 9", volume: 1450000 },
    { date: "Dec 10", volume: 1300000 },
    { date: "Dec 11", volume: 1800000 },
    { date: "Dec 12", volume: 1650000 },
    { date: "Dec 13", volume: 2100000 },
    { date: "Dec 14", volume: 1900000 },
  ]

  const bridgeTimeData = [
    { range: "0-10s", count: 450 },
    { range: "10-20s", count: 680 },
    { range: "20-30s", count: 320 },
    { range: "30-40s", count: 85 },
    { range: "40-50s", count: 25 },
    { range: "50-60s", count: 12 },
  ]

  const solverPerformance = [
    {
      rank: 1,
      solver: "0x1234...5678",
      volume: "$4.2M",
      fills: 1234,
      avgTime: "15s",
      successRate: "99.8%",
    },
    {
      rank: 2,
      solver: "0xabcd...ef12",
      volume: "$3.8M",
      fills: 1089,
      avgTime: "18s",
      successRate: "99.5%",
    },
    {
      rank: 3,
      solver: "0x9876...4321",
      volume: "$2.9M",
      fills: 876,
      avgTime: "16s",
      successRate: "99.7%",
    },
    {
      rank: 4,
      solver: "0xfedc...ba98",
      volume: "$1.5M",
      fills: 543,
      avgTime: "22s",
      successRate: "98.9%",
    },
  ]

  // Calculate asset distribution from volume by token
  const assetDistribution = useMemo(() => {
    const colors = ["#f97316", "#ec4899", "#8b5cf6", "#06b6d4", "#10b981"];
    return Object.entries(volumeByToken)
      .map(([token, volume], index) => ({
        name: token,
        value: parseFloat(volume) || 0,
        color: colors[index % colors.length],
      }))
      .filter((asset) => asset.value > 0);
  }, [volumeByToken])

  const metrics = useMemo(
    () => [
      {
        title: "Total Transactions",
        value: statsLoading ? "..." : formatLargeNumber(totalIntents),
        change: statsLoading ? "" : `${completedIntents} completed`,
        trend: "up",
        icon: Activity,
      },
      {
        title: "Completed",
        value: statsLoading ? "..." : formatLargeNumber(completedIntents),
        change: statsLoading ? "" : `${filledIntents} filled`,
        trend: "up",
        icon: CheckCircle2,
      },
      {
        title: "Success Rate",
        value: statsLoading ? "..." : `${successRate.toFixed(1)}%`,
        change: statsLoading ? "" : `${failedIntents} failed`,
        trend: "up",
        icon: TrendingUp,
      },
      {
        title: "Pending",
        value: statsLoading ? "..." : formatLargeNumber(pendingIntents),
        change: statsLoading ? "" : `${refundedIntents} refunded`,
        trend: "up",
        icon: Clock,
      },
      {
        title: "ETH → Mantle",
        value: statsLoading ? "..." : formatLargeNumber(ethereumToMantle),
        change: statsLoading ? "" : "cross-chain",
        trend: "up",
        icon: ArrowDownRight,
      },
      {
        title: "Mantle → ETH",
        value: statsLoading ? "..." : formatLargeNumber(mantleToEthereum),
        change: statsLoading ? "" : "cross-chain",
        trend: "up",
        icon: ArrowUpRight,
      },
    ],
    [
      statsLoading,
      totalIntents,
      completedIntents,
      filledIntents,
      successRate,
      failedIntents,
      pendingIntents,
      refundedIntents,
      ethereumToMantle,
      mantleToEthereum,
    ]
  )

  return (
    <div className="min-h-screen bg-black">
      <Navigation />

      {/* Main Content */}
      <main className="px-4 pb-12 pt-24 sm:px-6">
        <div className="mx-auto max-w-7xl">
          {/* Header */}
          <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h1 className="mb-2 text-3xl font-bold text-white sm:text-4xl">Protocol Analytics</h1>
              <p className="text-neutral-400">Real-time metrics and performance data</p>
            </div>

            <div className="flex gap-3">
              <Button
                onClick={() => {
                  refetchStats()
                  refetchHealth()
                }}
                disabled={statsLoading || healthLoading}
                variant="outline"
                className="border-neutral-700 bg-neutral-800 hover:bg-neutral-700"
              >
                <RefreshCw className={`mr-2 h-4 w-4 ${statsLoading || healthLoading ? "animate-spin" : ""}`} />
                Refresh
              </Button>

              {/* Time Period Selector */}
              <Tabs value={timePeriod} onValueChange={setTimePeriod} className="w-full sm:w-auto">
                <TabsList className="border border-neutral-800 bg-neutral-900">
                  <TabsTrigger value="24h">24H</TabsTrigger>
                  <TabsTrigger value="7d">7D</TabsTrigger>
                  <TabsTrigger value="30d">30D</TabsTrigger>
                  <TabsTrigger value="all">ALL</TabsTrigger>
                </TabsList>
              </Tabs>
            </div>
          </div>

          {/* Error States */}
          {statsError && (
            <div className="mb-6 flex items-center gap-3 rounded-lg border border-red-500/20 bg-red-500/10 p-4 text-red-500">
              <AlertCircle className="h-5 w-5" />
              <div>
                <p className="font-medium">Error loading statistics</p>
                <p className="text-sm text-red-400">{statsError}</p>
              </div>
            </div>
          )}
          {healthError && (
            <div className="mb-6 flex items-center gap-3 rounded-lg border border-yellow-500/20 bg-yellow-500/10 p-4 text-yellow-500">
              <AlertCircle className="h-5 w-5" />
              <div>
                <p className="font-medium">Error loading health status</p>
                <p className="text-sm text-yellow-400">{healthError}</p>
              </div>
            </div>
          )}

          {/* Key Metrics Grid */}
          <div className="mb-8 grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {metrics.map((metric, index) => (
              <div
                key={index}
                className="rounded-lg border border-neutral-800 bg-neutral-900 p-6 transition-all duration-300 hover:border-orange-500/50"
              >
                <div className="mb-4 flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-orange-500/10">
                    <metric.icon className="h-5 w-5 text-orange-500" />
                  </div>
                  <div
                    className={`flex items-center gap-1 text-sm ${metric.trend === "up" ? "text-green-500" : "text-red-500"}`}
                  >
                    {metric.trend === "up" ? (
                      <ArrowUpRight className="h-4 w-4" />
                    ) : (
                      <ArrowDownRight className="h-4 w-4" />
                    )}
                    <span>{metric.change}</span>
                  </div>
                </div>
                <div className="mb-1 text-2xl font-bold text-white">{metric.value}</div>
                <div className="text-sm text-neutral-500">{metric.title}</div>
              </div>
            ))}
          </div>

          {/* Charts Grid */}
          <div className="mb-8 grid grid-cols-1 gap-6 lg:grid-cols-2">
            {/* Volume Over Time */}
            <Card className="border-neutral-800 bg-neutral-900">
              <CardHeader>
                <CardTitle className="text-white">Volume Over Time</CardTitle>
                <CardDescription className="text-neutral-400">Daily bridge volume in USD</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <LineChart data={volumeData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#404040" />
                    <XAxis dataKey="date" stroke="#737373" />
                    <YAxis stroke="#737373" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "#171717",
                        border: "1px solid #404040",
                        borderRadius: "8px",
                      }}
                      labelStyle={{ color: "#fff" }}
                      itemStyle={{ color: "#fff" }}
                      formatter={(value: any) => [`$${(value / 1000000).toFixed(2)}M`, "Volume"]}
                    />
                    <Line type="monotone" dataKey="volume" stroke="#f97316" strokeWidth={2} dot={{ fill: "#f97316" }} />
                  </LineChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            {/* Bridge Time Distribution */}
            <Card className="border-neutral-800 bg-neutral-900">
              <CardHeader>
                <CardTitle className="text-white">Bridge Time Distribution</CardTitle>
                <CardDescription className="text-neutral-400">Settlement time ranges</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <BarChart data={bridgeTimeData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#404040" />
                    <XAxis dataKey="range" stroke="#737373" />
                    <YAxis stroke="#737373" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "#171717",
                        border: "1px solid #404040",
                        borderRadius: "8px",
                      }}
                      labelStyle={{ color: "#fff" }}
                      itemStyle={{ color: "#fff" }}
                    />
                    <Bar dataKey="count" fill="#f97316" radius={[8, 8, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            {/* Asset Distribution */}
            <Card className="border-neutral-800 bg-neutral-900">
              <CardHeader>
                <CardTitle className="text-white">Top Assets Bridged</CardTitle>
                <CardDescription className="text-neutral-400">By volume percentage</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <PieChart>
                    <Pie
                      data={assetDistribution}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                      outerRadius={100}
                      fill="#8884d8"
                      dataKey="value"
                    >
                      {assetDistribution.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "#171717",
                        border: "1px solid #404040",
                        borderRadius: "8px",
                      }}
                      labelStyle={{ color: "#fff" }}
                      itemStyle={{ color: "#fff" }}
                    />
                  </PieChart>
                </ResponsiveContainer>
                <div className="mt-4 flex justify-center gap-4">
                  {assetDistribution.map((asset, index) => (
                    <div key={index} className="flex items-center gap-2">
                      <div className="h-3 w-3 rounded-full" style={{ backgroundColor: asset.color }}></div>
                      <span className="text-sm text-neutral-400">{asset.name}</span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            {/* Solver Performance Table */}
            <Card className="border-neutral-800 bg-neutral-900">
              <CardHeader>
                <CardTitle className="text-white">Top Solver Performance</CardTitle>
                <CardDescription className="text-neutral-400">Ranked by volume handled</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {solverPerformance.map((solver) => (
                    <div
                      key={solver.rank}
                      className="flex items-center justify-between rounded-lg bg-neutral-800/50 p-3 transition-colors hover:bg-neutral-800"
                    >
                      <div className="flex items-center gap-3">
                        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10 text-sm font-bold text-orange-500">
                          #{solver.rank}
                        </div>
                        <div>
                          <div className="font-mono text-sm text-white">{solver.solver}</div>
                          <div className="text-xs text-neutral-500">
                            {solver.fills} fills • {solver.avgTime} avg
                          </div>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-semibold text-white">{solver.volume}</div>
                        <div className="text-xs text-green-500">{solver.successRate}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Network Health Status */}
          <Card className="border-neutral-800 bg-neutral-900">
            <CardHeader>
              <CardTitle className="text-white">Network Health</CardTitle>
              <CardDescription className="text-neutral-400">
                Real-time system status
                {healthLoading && (
                  <span className="ml-2 inline-flex items-center gap-1 text-xs text-neutral-500">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    Loading...
                  </span>
                )}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {Object.entries(healthComponents).map(([component, status], index) => (
                  <div key={index} className="flex items-center justify-between rounded-lg bg-neutral-800/50 p-4">
                    <div>
                      <div className="mb-1 font-medium capitalize text-white">{component.replace(/_/g, " ")}</div>
                      <div className="text-xs text-neutral-500">
                        {status === "healthy" ? "Operational" : status}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <div
                        className={`h-2 w-2 animate-pulse rounded-full ${
                          status === "healthy" ? "bg-green-500" : "bg-yellow-500"
                        }`}
                      ></div>
                      <Badge
                        variant="outline"
                        className={
                          status === "healthy"
                            ? "border-green-500/20 bg-green-500/10 text-green-500"
                            : "border-yellow-500/20 bg-yellow-500/10 text-yellow-500"
                        }
                      >
                        {status}
                      </Badge>
                    </div>
                  </div>
                ))}
                {Object.keys(healthComponents).length === 0 && !healthLoading && (
                  <div className="col-span-full py-8 text-center text-neutral-500">
                    No health data available
                  </div>
                )}
              </div>
              <div className="mt-4 flex items-center justify-between border-t border-neutral-800 pt-4">
                <div className="text-sm text-neutral-500">
                  Overall Status:{" "}
                  <span
                    className={`font-medium ${healthStatus === "healthy" ? "text-green-500" : "text-yellow-500"}`}
                  >
                    {healthStatus || "Unknown"}
                  </span>
                </div>
                <div className="text-xs text-neutral-600">Auto-refresh every 15s</div>
              </div>
            </CardContent>
          </Card>
        </div>
      </main>
    </div>
  )
}
