"use client"

import { motion } from "framer-motion"
import Navigation from "@/components/shared/Navigation"
import Footer from "@/components/shared/Footer"
import BridgeForm from "@/components/bridge/BridgeForm"
import { TrendingUp, Users, Zap, Shield } from "lucide-react"

export default function BridgePage() {
  const stats = [
    { label: "24h Volume", value: "$12.5M", icon: TrendingUp, color: "text-green-500" },
    { label: "Total Bridges", value: "45,231", icon: Users, color: "text-blue-500" },
    { label: "Avg Speed", value: "18s", icon: Zap, color: "text-orange-500" },
    { label: "Success Rate", value: "99.4%", icon: Shield, color: "text-purple-500" },
  ]

  return (
    <div className="min-h-screen bg-black">
      <Navigation />

      {/* Main Content */}
      <main className="px-4 pb-12 pt-24 sm:px-6">
        <div className="mx-auto max-w-7xl">
          {/* Header */}
          <motion.div initial={{ opacity: 0, y: -20 }} animate={{ opacity: 1, y: 0 }} className="mb-12 text-center">
            <h1 className="mb-4 text-4xl font-bold text-white sm:text-5xl">
              Cross-Chain <span className="text-orange-500">Bridge</span>
            </h1>
            <p className="mx-auto max-w-2xl text-lg text-neutral-400">
              Transfer assets between Mantle and Ethereum with privacy-preserving technology
            </p>
          </motion.div>

          {/* Stats Bar */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="mb-12 grid grid-cols-2 gap-4 md:grid-cols-4"
          >
            {stats.map((stat, index) => {
              const Icon = stat.icon
              return (
                <div
                  key={index}
                  className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-4 text-center transition-colors hover:border-orange-500/30"
                >
                  <div className="mb-2 flex items-center justify-center gap-2">
                    <Icon className={`h-4 w-4 ${stat.color}`} />
                    <span className="text-xs uppercase tracking-wider text-neutral-500">{stat.label}</span>
                  </div>
                  <div className="text-2xl font-bold text-white">{stat.value}</div>
                </div>
              )
            })}
          </motion.div>

          {/* Bridge Form */}
          <BridgeForm />

          {/* Info Cards */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
            className="mt-12 grid grid-cols-1 gap-6 md:grid-cols-3"
          >
            {[
              {
                title: "Privacy First",
                description: "Your transactions are private using zero-knowledge commitments",
                icon: Shield,
              },
              {
                title: "Lightning Fast",
                description: "Bridge completes in 10-30 seconds with our solver network",
                icon: Zap,
              },
              {
                title: "Low Fees",
                description: "Only 0.15% total fee, much lower than traditional bridges",
                icon: TrendingUp,
              },
            ].map((card, index) => {
              const Icon = card.icon
              return (
                <div
                  key={index}
                  className="rounded-lg border border-neutral-800 bg-neutral-900/30 p-6 transition-colors hover:border-orange-500/30"
                >
                  <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-orange-500/10">
                    <Icon className="h-6 w-6 text-orange-500" />
                  </div>
                  <h3 className="mb-2 text-lg font-semibold text-white">{card.title}</h3>
                  <p className="text-sm text-neutral-400">{card.description}</p>
                </div>
              )
            })}
          </motion.div>
        </div>
      </main>

      <Footer />
    </div>
  )
}
