"use client"

import { useState, useEffect } from "react"
import Link from "next/link"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ArrowRight, TrendingUp, Users, Clock, CheckCircle2, FileText } from "lucide-react"

export default function HeroSection() {
  const [totalVolume, setTotalVolume] = useState(0)
  const [activeUsers, setActiveUsers] = useState(0)
  const [avgBridgeTime, setAvgBridgeTime] = useState(0)
  const [totalTransactions, setTotalTransactions] = useState(0)

  // Animated counter effect
  useEffect(() => {
    const volumeTarget = 12500000
    const usersTarget = 3421
    const timeTarget = 18
    const transactionsTarget = 45231
    const duration = 2000

    const stepVolume = volumeTarget / (duration / 16)
    const stepUsers = usersTarget / (duration / 16)
    const stepTime = timeTarget / (duration / 16)
    const stepTransactions = transactionsTarget / (duration / 16)

    let currentVolume = 0
    let currentUsers = 0
    let currentTime = 0
    let currentTransactions = 0

    const interval = setInterval(() => {
      currentVolume = Math.min(currentVolume + stepVolume, volumeTarget)
      currentUsers = Math.min(currentUsers + stepUsers, usersTarget)
      currentTime = Math.min(currentTime + stepTime, timeTarget)
      currentTransactions = Math.min(currentTransactions + stepTransactions, transactionsTarget)

      setTotalVolume(Math.floor(currentVolume))
      setActiveUsers(Math.floor(currentUsers))
      setAvgBridgeTime(Math.floor(currentTime))
      setTotalTransactions(Math.floor(currentTransactions))

      if (currentVolume >= volumeTarget) {
        clearInterval(interval)
      }
    }, 16)

    return () => clearInterval(interval)
  }, [])

  const formatNumber = (num: number) => {
    if (num >= 1000000) {
      return `$${(num / 1000000).toFixed(1)}M`
    }
    return num.toLocaleString()
  }

  return (
    <section className="relative pt-32 pb-20 sm:pt-40 sm:pb-32 px-4 sm:px-6 overflow-hidden">
      {/* Animated Background Grid */}
      <div
        className="absolute inset-0 opacity-10"
        style={{
          backgroundImage: `linear-gradient(rgba(245, 115, 22, 0.3) 1px, transparent 1px), linear-gradient(90deg, rgba(245, 115, 22, 0.3) 1px, transparent 1px)`,
          backgroundSize: "50px 50px",
          animation: "grid-move 20s linear infinite",
        }}
      />

      {/* Glowing Orbs */}
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-orange-500/10 rounded-full blur-3xl animate-pulse"></div>
      <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-pink-500/10 rounded-full blur-3xl animate-pulse delay-1000"></div>

      <div className="max-w-6xl mx-auto text-center relative z-10">
        {/* Badge */}
        <Badge
          variant="outline"
          className="mb-6 border-orange-500/50 text-orange-500 px-4 py-1.5 text-xs tracking-wider"
        >
          <span className="animate-pulse mr-2">●</span>
          PRIVACY-ENHANCED CROSS-CHAIN BRIDGE
        </Badge>

        {/* Main Headline */}
        <h1 className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight mb-6 leading-tight">
          <span className="text-white">Bridge Assets</span>
          <br />
          <span className="text-white">Across Chains.</span>
          <br />
          <span className="bg-gradient-to-r from-orange-500 via-pink-500 to-orange-500 bg-clip-text text-transparent animate-gradient">
            Privately. Instantly.
          </span>
        </h1>

        {/* Subtitle */}
        <p className="text-neutral-400 text-lg sm:text-xl mb-10 max-w-3xl mx-auto leading-relaxed">
          Privacy-preserving, intent-based bridge with one-click UX and automatic claim execution. Built on
          battle-tested technology.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-4 justify-center mb-16">
          <Link href="/bridge">
            <Button
              size="lg"
              className="bg-orange-500 hover:bg-orange-600 text-white shadow-2xl shadow-orange-500/30 transition-all duration-300 hover:scale-105 px-8 text-base group"
            >
              Launch App
              <ArrowRight className="ml-2 w-4 h-4 group-hover:translate-x-1 transition-transform" />
            </Button>
          </Link>
          <Link href="/docs">
            <Button
              size="lg"
              variant="outline"
              className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800 text-white px-8 text-base"
            >
              <FileText className="mr-2 w-4 h-4" />
              Read Docs
            </Button>
          </Link>
        </div>

        {/* Stats Ticker */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-6 max-w-4xl mx-auto">
          <div className="bg-neutral-900/50 border border-neutral-800 rounded-lg p-6 backdrop-blur-sm hover:border-orange-500/30 transition-all duration-300">
            <div className="flex items-center justify-center gap-2 mb-2">
              <TrendingUp className="w-5 h-5 text-orange-500" />
              <span className="text-xs text-neutral-500 tracking-wider">TOTAL VOLUME</span>
            </div>
            <div className="text-2xl sm:text-3xl font-bold text-white font-mono">{formatNumber(totalVolume)}</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-lg p-6 backdrop-blur-sm hover:border-orange-500/30 transition-all duration-300">
            <div className="flex items-center justify-center gap-2 mb-2">
              <Users className="w-5 h-5 text-orange-500" />
              <span className="text-xs text-neutral-500 tracking-wider">ACTIVE USERS</span>
            </div>
            <div className="text-2xl sm:text-3xl font-bold text-white font-mono">{activeUsers.toLocaleString()}</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-lg p-6 backdrop-blur-sm hover:border-orange-500/30 transition-all duration-300">
            <div className="flex items-center justify-center gap-2 mb-2">
              <Clock className="w-5 h-5 text-orange-500" />
              <span className="text-xs text-neutral-500 tracking-wider">AVG BRIDGE TIME</span>
            </div>
            <div className="text-2xl sm:text-3xl font-bold text-white font-mono">{avgBridgeTime}s</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-lg p-6 backdrop-blur-sm hover:border-orange-500/30 transition-all duration-300">
            <div className="flex items-center justify-center gap-2 mb-2">
              <CheckCircle2 className="w-5 h-5 text-orange-500" />
              <span className="text-xs text-neutral-500 tracking-wider">TRANSACTIONS</span>
            </div>
            <div className="text-2xl sm:text-3xl font-bold text-white font-mono">
              {totalTransactions.toLocaleString()}
            </div>
          </div>
        </div>
      </div>

      <style jsx>{`
        @keyframes grid-move {
          0% {
            transform: translateY(0);
          }
          100% {
            transform: translateY(50px);
          }
        }
        @keyframes gradient {
          0%,
          100% {
            background-position: 0% 50%;
          }
          50% {
            background-position: 100% 50%;
          }
        }
        .animate-gradient {
          background-size: 200% 200%;
          animation: gradient 3s ease infinite;
        }
        .delay-1000 {
          animation-delay: 1s;
        }
      `}</style>
    </section>
  )
}
