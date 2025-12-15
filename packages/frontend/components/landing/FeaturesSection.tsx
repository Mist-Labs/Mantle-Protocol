"use client"

import { Shield, Zap, DollarSign, MousePointerClick, Network, Lock } from "lucide-react"
import { Badge } from "@/components/ui/badge"

export default function FeaturesSection() {
  const features = [
    {
      icon: Shield,
      title: "Privacy First",
      description: "Commitment-based architecture ensures your transactions remain completely private on-chain",
      gradient: "from-orange-500/20 to-pink-500/20",
    },
    {
      icon: Zap,
      title: "Lightning Fast",
      description: "Intent-based solver network provides near-instant bridging in 10-30 seconds",
      gradient: "from-yellow-500/20 to-orange-500/20",
    },
    {
      icon: DollarSign,
      title: "Lowest Fees",
      description: "Competitive solver market keeps costs minimal at just 0.15% total fee",
      gradient: "from-green-500/20 to-emerald-500/20",
    },
    {
      icon: MousePointerClick,
      title: "One-Click UX",
      description: "Auto-claiming technology eliminates manual steps for seamless bridging experience",
      gradient: "from-blue-500/20 to-cyan-500/20",
    },
    {
      icon: Network,
      title: "ERC-7683 Compatible",
      description: "Access to major solver networks including Across, UniswapX, and more",
      gradient: "from-purple-500/20 to-pink-500/20",
    },
    {
      icon: Lock,
      title: "Battle-Tested Security",
      description: "Built on proven technologies: Tornado Cash commitments, Across Protocol intents",
      gradient: "from-red-500/20 to-orange-500/20",
    },
  ]

  return (
    <section className="bg-gradient-to-b from-black to-neutral-950 px-4 py-20 sm:px-6">
      <div className="mx-auto max-w-7xl">
        <div className="mb-16 text-center">
          <Badge variant="outline" className="mb-4 border-orange-500/50 tracking-wider text-orange-500">
            FEATURES
          </Badge>
          <h2 className="mb-4 text-3xl font-bold text-white sm:text-4xl md:text-5xl">
            Built for <span className="text-orange-500">Privacy</span> & <span className="text-orange-500">Speed</span>
          </h2>
          <p className="mx-auto max-w-2xl text-lg text-neutral-400">
            Combining zero-knowledge privacy with lightning-fast settlement through an intent-based solver network
          </p>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, index) => (
            <div
              key={index}
              className="group relative overflow-hidden rounded-xl border border-neutral-800 bg-neutral-900 p-8 transition-all duration-300 hover:scale-105 hover:border-orange-500/50"
            >
              {/* Gradient Background */}
              <div
                className={`absolute inset-0 bg-gradient-to-br ${feature.gradient} opacity-0 transition-opacity duration-300 group-hover:opacity-100`}
              ></div>

              <div className="relative z-10">
                <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-orange-500/10 transition-colors group-hover:bg-orange-500/20">
                  <feature.icon className="h-6 w-6 text-orange-500" />
                </div>
                <h3 className="mb-3 text-xl font-bold text-white">{feature.title}</h3>
                <p className="leading-relaxed text-neutral-400">{feature.description}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
