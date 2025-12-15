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
    <section className="relative overflow-hidden px-4 pb-20 pt-32 sm:px-6 sm:pb-32 sm:pt-40">
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
      <div className="absolute left-1/4 top-1/4 h-96 w-96 animate-pulse rounded-full bg-orange-500/10 blur-3xl"></div>
      <div className="absolute bottom-1/4 right-1/4 h-96 w-96 animate-pulse rounded-full bg-pink-500/10 blur-3xl delay-1000"></div>

      <div className="relative z-10 mx-auto max-w-6xl text-center">
        {/* Badge */}
        <Badge
          variant="outline"
          className="mb-6 border-orange-500/50 px-4 py-1.5 text-xs tracking-wider text-orange-500"
        >
          <span className="mr-2 animate-pulse">●</span>
          PRIVACY-ENHANCED CROSS-CHAIN BRIDGE
        </Badge>

        {/* Main Headline */}
        <h1 className="mb-6 text-4xl font-bold leading-tight tracking-tight sm:text-5xl md:text-6xl lg:text-7xl">
          <span className="text-white">Bridge Assets</span>
          <br />
          <span className="text-white">Across Chains.</span>
          <br />
          <span className="animate-gradient bg-gradient-to-r from-orange-500 via-pink-500 to-orange-500 bg-clip-text text-transparent">
            Privately. Instantly.
          </span>
        </h1>

        {/* Subtitle */}
        <p className="mx-auto mb-10 max-w-3xl text-lg leading-relaxed text-neutral-400 sm:text-xl">
          Privacy-preserving, intent-based bridge with one-click UX and automatic claim execution. Built on
          battle-tested technology.
        </p>

        {/* CTA Buttons */}
        <div className="mb-16 flex flex-col justify-center gap-4 sm:flex-row">
          <Link href="/bridge">
            <Button
              size="lg"
              className="group bg-orange-500 px-8 text-base text-white shadow-2xl shadow-orange-500/30 transition-all duration-300 hover:scale-105 hover:bg-orange-600"
            >
              Launch App
              <ArrowRight className="ml-2 h-4 w-4 transition-transform group-hover:translate-x-1" />
            </Button>
          </Link>
          <Link href="/docs">
            <Button
              size="lg"
              variant="outline"
              className="border-neutral-700 bg-neutral-900 px-8 text-base text-white hover:bg-neutral-800"
            >
              <FileText className="mr-2 h-4 w-4" />
              Read Docs
            </Button>
          </Link>
        </div>

        {/* Stats Ticker */}
        <div className="mx-auto grid max-w-4xl grid-cols-2 gap-6 md:grid-cols-4">
          <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm transition-all duration-300 hover:border-orange-500/30">
            <div className="mb-2 flex items-center justify-center gap-2">
              <TrendingUp className="h-5 w-5 text-orange-500" />
              <span className="text-xs tracking-wider text-neutral-500">TOTAL VOLUME</span>
            </div>
            <div className="font-mono text-2xl font-bold text-white sm:text-3xl">{formatNumber(totalVolume)}</div>
          </div>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm transition-all duration-300 hover:border-orange-500/30">
            <div className="mb-2 flex items-center justify-center gap-2">
              <Users className="h-5 w-5 text-orange-500" />
              <span className="text-xs tracking-wider text-neutral-500">ACTIVE USERS</span>
            </div>
            <div className="font-mono text-2xl font-bold text-white sm:text-3xl">{activeUsers.toLocaleString()}</div>
          </div>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm transition-all duration-300 hover:border-orange-500/30">
            <div className="mb-2 flex items-center justify-center gap-2">
              <Clock className="h-5 w-5 text-orange-500" />
              <span className="text-xs tracking-wider text-neutral-500">AVG BRIDGE TIME</span>
            </div>
            <div className="font-mono text-2xl font-bold text-white sm:text-3xl">{avgBridgeTime}s</div>
          </div>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 backdrop-blur-sm transition-all duration-300 hover:border-orange-500/30">
            <div className="mb-2 flex items-center justify-center gap-2">
              <CheckCircle2 className="h-5 w-5 text-orange-500" />
              <span className="text-xs tracking-wider text-neutral-500">TRANSACTIONS</span>
            </div>
            <div className="font-mono text-2xl font-bold text-white sm:text-3xl">
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
