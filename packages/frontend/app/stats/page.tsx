"use client"

import { useState } from "react"
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
} from "lucide-react"
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

  // Mock data for charts
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

  const assetDistribution = [
    { name: "ETH", value: 45, color: "#f97316" },
    { name: "USDT", value: 25, color: "#ec4899" },
    { name: "USDC", value: 20, color: "#8b5cf6" },
    { name: "MNT", value: 10, color: "#06b6d4" },
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

  const metrics = [
    {
      title: "Total Volume (24h)",
      value: "$12.5M",
      change: "+15.3%",
      trend: "up",
      icon: DollarSign,
    },
    {
      title: "Total Transactions",
      value: "45,231",
      change: "+8.7%",
      trend: "up",
      icon: Activity,
    },
    {
      title: "Average Bridge Time",
      value: "18s",
      change: "-12.5%",
      trend: "up",
      icon: Clock,
    },
    {
      title: "Success Rate",
      value: "99.4%",
      change: "+0.3%",
      trend: "up",
      icon: CheckCircle2,
    },
    {
      title: "Active Users",
      value: "3,421",
      change: "+22.1%",
      trend: "up",
      icon: Users,
    },
    {
      title: "Active Solvers",
      value: "47",
      change: "+5",
      trend: "up",
      icon: Zap,
    },
  ]

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
              <CardDescription className="text-neutral-400">Real-time system status</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {[
                  { name: "Mantle RPC", status: "operational", latency: "45ms" },
                  { name: "Ethereum RPC", status: "operational", latency: "82ms" },
                  { name: "Relayer", status: "operational", uptime: "99.98%" },
                  { name: "Solver Network", status: "operational", active: "47" },
                ].map((service, index) => (
                  <div key={index} className="flex items-center justify-between rounded-lg bg-neutral-800/50 p-4">
                    <div>
                      <div className="mb-1 font-medium text-white">{service.name}</div>
                      <div className="text-xs text-neutral-500">
                        {service.latency || service.uptime || `${service.active} active`}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <div className="h-2 w-2 animate-pulse rounded-full bg-green-500"></div>
                      <Badge variant="outline" className="border-green-500/20 bg-green-500/10 text-green-500">
                        {service.status}
                      </Badge>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      </main>
    </div>
  )
}
