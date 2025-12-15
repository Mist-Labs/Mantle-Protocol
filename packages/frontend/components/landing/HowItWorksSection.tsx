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
    <section className="bg-black px-4 py-20 sm:px-6">
      <div className="mx-auto max-w-6xl">
        <div className="mb-16 text-center">
          <Badge variant="outline" className="mb-4 border-orange-500/50 tracking-wider text-orange-500">
            HOW IT WORKS
          </Badge>
          <h2 className="mb-4 text-3xl font-bold text-white sm:text-4xl md:text-5xl">
            Bridge in <span className="text-orange-500">4 Simple Steps</span>
          </h2>
          <p className="mx-auto max-w-2xl text-lg text-neutral-400">
            From wallet connection to funds received in under 30 seconds
          </p>
        </div>

        <div className="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4">
          {steps.map((step, index) => (
            <div key={index} className="relative">
              {/* Connector Line (desktop only) */}
              {index < steps.length - 1 && (
                <div className="absolute left-full top-12 hidden h-0.5 w-full bg-gradient-to-r from-orange-500/50 to-transparent lg:block"></div>
              )}

              <div className="relative flex h-[280px] flex-col rounded-xl border border-neutral-800 bg-neutral-900 p-6 transition-all duration-300 hover:border-orange-500/50">
                {/* Step Number */}
                <div className="mb-4 text-6xl font-bold text-orange-500/20">{step.number}</div>

                {/* Icon */}
                <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-orange-500/10">
                  <step.icon className="h-6 w-6 text-orange-500" />
                </div>

                {/* Content */}
                <h3 className="mb-2 text-lg font-bold text-white">{step.title}</h3>
                <p className="text-sm text-neutral-400">{step.description}</p>
              </div>
            </div>
          ))}
        </div>

        <div className="mt-12 text-center">
          <Link href="/bridge">
            <Button
              size="lg"
              className="bg-orange-500 px-8 text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105 hover:bg-orange-600"
            >
              Try It Now
              <ArrowRight className="ml-2 h-4 w-4" />
            </Button>
          </Link>
        </div>
      </div>
    </section>
  )
}
