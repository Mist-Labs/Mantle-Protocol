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
    <section className="py-20 px-4 sm:px-6 bg-gradient-to-b from-black to-neutral-950">
      <div className="max-w-7xl mx-auto">
        <div className="text-center mb-16">
          <Badge variant="outline" className="mb-4 border-orange-500/50 text-orange-500 tracking-wider">
            FEATURES
          </Badge>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-white mb-4">
            Built for <span className="text-orange-500">Privacy</span> & <span className="text-orange-500">Speed</span>
          </h2>
          <p className="text-neutral-400 text-lg max-w-2xl mx-auto">
            Combining zero-knowledge privacy with lightning-fast settlement through an intent-based solver network
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feature, index) => (
            <div
              key={index}
              className="group relative bg-neutral-900 border border-neutral-800 rounded-xl p-8 hover:border-orange-500/50 transition-all duration-300 hover:scale-105 overflow-hidden"
            >
              {/* Gradient Background */}
              <div
                className={`absolute inset-0 bg-gradient-to-br ${feature.gradient} opacity-0 group-hover:opacity-100 transition-opacity duration-300`}
              ></div>

              <div className="relative z-10">
                <div className="w-12 h-12 bg-orange-500/10 rounded-lg flex items-center justify-center mb-4 group-hover:bg-orange-500/20 transition-colors">
                  <feature.icon className="w-6 h-6 text-orange-500" />
                </div>
                <h3 className="text-xl font-bold text-white mb-3">{feature.title}</h3>
                <p className="text-neutral-400 leading-relaxed">{feature.description}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
