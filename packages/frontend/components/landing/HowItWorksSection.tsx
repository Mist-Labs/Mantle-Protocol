"use client"

import Link from "next/link"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { MousePointerClick, Lock, Network, CheckCircle2, ArrowRight } from "lucide-react"

export default function HowItWorksSection() {
  const steps = [
    {
      number: "01",
      title: "Connect Wallet",
      description: "Connect your Web3 wallet securely with one click",
      icon: MousePointerClick,
    },
    {
      number: "02",
      title: "Create Intent",
      description: "Generate privacy commitment and submit bridge intent",
      icon: Lock,
    },
    {
      number: "03",
      title: "Solver Fills",
      description: "Competitive solvers provide instant liquidity on destination chain",
      icon: Network,
    },
    {
      number: "04",
      title: "Auto-Claim",
      description: "Funds automatically claimed and sent to your wallet",
      icon: CheckCircle2,
    },
  ]

  return (
    <section className="py-20 px-4 sm:px-6 bg-black">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-16">
          <Badge variant="outline" className="mb-4 border-orange-500/50 text-orange-500 tracking-wider">
            HOW IT WORKS
          </Badge>
          <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-white mb-4">
            Bridge in <span className="text-orange-500">4 Simple Steps</span>
          </h2>
          <p className="text-neutral-400 text-lg max-w-2xl mx-auto">
            From wallet connection to funds received in under 30 seconds
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
          {steps.map((step, index) => (
            <div key={index} className="relative">
              {/* Connector Line (desktop only) */}
              {index < steps.length - 1 && (
                <div className="hidden lg:block absolute top-12 left-full w-full h-0.5 bg-gradient-to-r from-orange-500/50 to-transparent"></div>
              )}

              <div className="relative bg-neutral-900 border border-neutral-800 rounded-xl p-6 hover:border-orange-500/50 transition-all duration-300">
                {/* Step Number */}
                <div className="text-6xl font-bold text-orange-500/20 mb-4">{step.number}</div>

                {/* Icon */}
                <div className="w-12 h-12 bg-orange-500/10 rounded-lg flex items-center justify-center mb-4">
                  <step.icon className="w-6 h-6 text-orange-500" />
                </div>

                {/* Content */}
                <h3 className="text-lg font-bold text-white mb-2">{step.title}</h3>
                <p className="text-neutral-400 text-sm">{step.description}</p>
              </div>
            </div>
          ))}
        </div>

        <div className="text-center mt-12">
          <Link href="/bridge">
            <Button
              size="lg"
              className="bg-orange-500 hover:bg-orange-600 text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105 px-8"
            >
              Try It Now
              <ArrowRight className="ml-2 w-4 h-4" />
            </Button>
          </Link>
        </div>
      </div>
    </section>
  )
}
