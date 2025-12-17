"use client"

import { useState } from "react"
import Navigation from "@/components/shared/Navigation"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  ChevronRight,
  BookOpen,
  Shield,
  Zap,
  Code,
  FileText,
  ExternalLink,
  Github,
  MessageCircle,
  Search,
} from "lucide-react"

export default function DocsPage() {
  const [activeSection, setActiveSection] = useState("getting-started")
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  const navigation = [
    {
      title: "Getting Started",
      items: [
        { id: "introduction", title: "Introduction", icon: BookOpen },
        { id: "getting-started", title: "Quick Start", icon: Zap },
        { id: "how-it-works", title: "How It Works", icon: Shield },
      ],
    },
    {
      title: "Concepts",
      items: [
        { id: "privacy-model", title: "Privacy Model", icon: Shield },
        { id: "intent-based", title: "Intent-Based Architecture", icon: Code },
        { id: "solver-network", title: "Solver Network", icon: Zap },
      ],
    },
    {
      title: "Guides",
      items: [
        { id: "bridge-tokens", title: "How to Bridge Tokens", icon: FileText },
        { id: "become-solver", title: "Become a Solver", icon: Code },
        { id: "run-relayer", title: "Run a Relayer", icon: Code },
      ],
    },
  ]

  const content: Record<string, { title: string; content: React.ReactNode }> = {
    "getting-started": {
      title: "Quick Start",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Welcome to Shadow Swap! This guide will help you get started with privacy-preserving cross-chain bridging in
            minutes.
          </p>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 flex items-center gap-2 text-lg font-semibold text-white">
              <Zap className="h-5 w-5 text-orange-500" />
              What is Shadow Swap?
            </h3>
            <p className="text-neutral-300">
              Shadow Swap is a privacy-enhanced, intent-based bridge that enables secure token transfers between
              Ethereum L1 and Mantle L2 with automatic claim execution.
            </p>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Features</h3>
            <ul className="space-y-3">
              {[
                "Privacy-preserving commitments (zero-knowledge technology)",
                "Lightning-fast settlement (10-30 seconds)",
                "One-click UX with auto-claiming",
                "Competitive fees (0.15% total)",
                "ERC-7683 compatible solver network",
              ].map((feature, index) => (
                <li key={index} className="flex items-start gap-3">
                  <ChevronRight className="mt-0.5 h-5 w-5 flex-shrink-0 text-orange-500" />
                  <span className="text-neutral-300">{feature}</span>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Bridge Your First Transaction</h3>
            <ol className="space-y-4">
              {[
                { step: "1", text: "Connect your Web3 wallet (MetaMask, WalletConnect, etc.)" },
                { step: "2", text: "Select source and destination networks" },
                { step: "3", text: "Enter amount and destination address" },
                { step: "4", text: 'Click "Bridge Now" and approve the transaction' },
                { step: "5", text: "Funds arrive automatically in 10-30 seconds" },
              ].map((item) => (
                <li key={item.step} className="flex items-start gap-4">
                  <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                    {item.step}
                  </div>
                  <span className="mt-1 text-neutral-300">{item.text}</span>
                </li>
              ))}
            </ol>
          </div>

          <div className="flex gap-3">
            <Button className="bg-orange-500 hover:bg-orange-600">
              <ExternalLink className="mr-2 h-4 w-4" />
              Launch App
            </Button>
            <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800">
              <FileText className="mr-2 h-4 w-4" />
              View Tutorial
            </Button>
          </div>
        </div>
      ),
    },
    introduction: {
      title: "Introduction",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Shadow Swap is a privacy-preserving cross-chain bridge built on Mantle L2, designed to provide fast, secure,
            and truly private token transfers.
          </p>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Why Shadow Swap?</h3>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {[
                {
                  title: "Privacy by Design",
                  description: "Commitment-based architecture hides user identity and transaction details on-chain",
                },
                {
                  title: "Lightning Fast",
                  description: "Intent-based solver network provides 10-30 second settlement times",
                },
                {
                  title: "Lowest Fees",
                  description: "Competitive solver market keeps costs at just 0.15% vs. traditional 0.5-1%",
                },
                {
                  title: "Battle-Tested",
                  description: "Built on proven technologies: Tornado Cash commitments, Across Protocol intents",
                },
              ].map((item, index) => (
                <div
                  key={index}
                  className="rounded-lg border border-neutral-800 bg-neutral-900 p-6 transition-colors hover:border-orange-500/50"
                >
                  <h4 className="mb-2 font-semibold text-white">{item.title}</h4>
                  <p className="text-sm text-neutral-400">{item.description}</p>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">Supported Networks</h3>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-orange-500/10">
                  <span className="font-bold text-orange-500">M</span>
                </div>
                <div>
                  <div className="font-medium text-white">Mantle L2</div>
                  <div className="text-xs text-neutral-500">Optimized for low fees</div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-orange-500/10">
                  <span className="font-bold text-orange-500">Ξ</span>
                </div>
                <div>
                  <div className="font-medium text-white">Ethereum L1</div>
                  <div className="text-xs text-neutral-500">Maximum security</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      ),
    },
    "how-it-works": {
      title: "How It Works",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Shadow Swap uses an intent-based architecture with privacy-preserving commitments to enable fast, private
            cross-chain transfers.
          </p>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Architecture Overview</h3>
            <div className="space-y-4">
              {[
                {
                  step: "Intent Creation",
                  description:
                    "User creates a privacy commitment (Poseidon hash) and submits intent on source chain. Tokens are locked in the contract.",
                },
                {
                  step: "Solver Fulfillment",
                  description:
                    "Solvers monitor intents and provide liquidity on the destination chain, competing for best rates.",
                },
                {
                  step: "Root Synchronization",
                  description:
                    "Relayer syncs Merkle roots between chains every 2 minutes to enable cross-chain verification.",
                },
                {
                  step: "Auto-Claim",
                  description:
                    "Relayer automatically claims funds on behalf of user using pre-signed authorization. Funds released to destination address.",
                },
              ].map((item, index) => (
                <div key={index} className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
                  <div className="mb-3 flex items-center gap-3">
                    <div className="flex h-8 w-8 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                      {index + 1}
                    </div>
                    <h4 className="font-semibold text-white">{item.step}</h4>
                  </div>
                  <p className="text-neutral-400">{item.description}</p>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 flex items-center gap-2 text-lg font-semibold text-white">
              <Shield className="h-5 w-5 text-orange-500" />
              Privacy Guarantees
            </h3>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>User wallet address never touches on-chain</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Source and destination addresses encrypted</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>No visible link between deposit and withdrawal on-chain</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Commitment-based system prevents transaction correlation</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
    "privacy-model": {
      title: "Privacy Model",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Shadow Swap uses a commitment-based privacy model inspired by Tornado Cash to protect user privacy while
            maintaining verifiability.
          </p>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">Privacy Layers</h3>
            <div className="space-y-4">
              <div>
                <h4 className="mb-2 font-medium text-white">On-Chain Privacy</h4>
                <p className="text-sm text-neutral-400">
                  Commitment = Poseidon(secret, nullifier, amount, destChain). Only the commitment is visible on-chain,
                  hiding all user data.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Nullifier System</h4>
                <p className="text-sm text-neutral-400">
                  Prevents double-spending while maintaining privacy. Each withdrawal uses a unique nullifier derived
                  from the secret.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Relayer Privacy</h4>
                <p className="text-sm text-neutral-400">
                  Relayer batches operations to decorrelate deposits from withdrawals, adding timing privacy.
                </p>
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">What's Private vs. Public</h3>
            <div className="mt-4 grid grid-cols-1 gap-6 md:grid-cols-2">
              <div>
                <h4 className="mb-2 flex items-center gap-2 font-medium text-white">
                  <Shield className="h-4 w-4 text-green-500" />
                  Private (Off-Chain)
                </h4>
                <ul className="space-y-1 text-sm text-neutral-400">
                  <li>• User wallet address</li>
                  <li>• Source & destination addresses</li>
                  <li>• Secret & nullifier values</li>
                  <li>• Deposit-withdrawal correlation</li>
                </ul>
              </div>
              <div>
                <h4 className="mb-2 flex items-center gap-2 font-medium text-white">
                  <Code className="h-4 w-4 text-orange-500" />
                  Public (On-Chain)
                </h4>
                <ul className="space-y-1 text-sm text-neutral-400">
                  <li>• Intent commitment (hash)</li>
                  <li>• Intent filled event</li>
                  <li>• Nullifier used (no link to intent)</li>
                  <li>• Token amounts</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      ),
    },
    "intent-based": {
      title: "Intent-Based Architecture",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Shadow Swap uses an intent-based architecture where users express their desired outcome and a competitive
            solver network fulfills it, providing the best execution.
          </p>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">What is an Intent?</h3>
            <p className="text-neutral-400">
              An intent is a signed message expressing what a user wants to achieve (e.g., "Bridge 1 ETH from Ethereum
              to Mantle") without specifying how it should be executed. This abstraction allows for optimal execution
              paths and competitive fee markets.
            </p>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Intent Flow</h3>
            <div className="space-y-4">
              {[
                {
                  step: "User Creates Intent",
                  description:
                    "User signs an intent specifying source token, amount, destination chain, and privacy commitment. No tokens are transferred yet.",
                },
                {
                  step: "Intent Broadcast",
                  description:
                    "Intent is broadcast to the solver network via on-chain event or off-chain relay. All solvers see the opportunity simultaneously.",
                },
                {
                  step: "Solver Competition",
                  description:
                    "Multiple solvers compete to fulfill the intent by offering best rates and fastest execution. Market forces keep fees low.",
                },
                {
                  step: "Intent Fulfillment",
                  description:
                    "Winning solver locks source tokens and provides liquidity on destination chain. User receives funds almost instantly.",
                },
              ].map((item, index) => (
                <div key={index} className="flex gap-4">
                  <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                    {index + 1}
                  </div>
                  <div>
                    <h4 className="mb-1 font-semibold text-white">{item.step}</h4>
                    <p className="text-sm text-neutral-400">{item.description}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">Benefits of Intent-Based Design</h3>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Better UX: Users only sign once, solver handles complexity</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Lower fees: Competitive market drives down costs</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Faster execution: Solvers pre-position liquidity</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>MEV protection: Intent execution happens off-chain</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
    "solver-network": {
      title: "Solver Network",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            The solver network is a decentralized marketplace of liquidity providers who compete to fulfill user
            intents, ensuring optimal pricing and fast execution.
          </p>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">How Solvers Work</h3>
            <p className="text-neutral-400">
              Solvers are independent operators who monitor intents, provide liquidity on destination chains, and earn
              fees for successful fills. They compete on speed, price, and reliability to win business.
            </p>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Solver Economics</h3>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
                <h4 className="mb-2 font-semibold text-white">Revenue Sources</h4>
                <ul className="space-y-2 text-sm text-neutral-400">
                  <li>• Bridge fees (0.15% of volume)</li>
                  <li>• MEV capture opportunities</li>
                  <li>• Interest on float capital</li>
                  <li>• Reputation rewards</li>
                </ul>
              </div>
              <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
                <h4 className="mb-2 font-semibold text-white">Costs</h4>
                <ul className="space-y-2 text-sm text-neutral-400">
                  <li>• Gas fees on both chains</li>
                  <li>• Capital lockup costs</li>
                  <li>• Infrastructure expenses</li>
                  <li>• Slippage risk</li>
                </ul>
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">Becoming a Solver</h3>
            <p className="mb-4 text-neutral-300">Interested in running a solver? Here's what you need:</p>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Capital: Minimum 10 ETH for liquidity provisioning</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Infrastructure: Reliable RPC endpoints and monitoring</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Technical skills: Ability to run and maintain solver software</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Risk management: Understanding of DeFi risks and mitigation</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
    "bridge-tokens": {
      title: "How to Bridge Tokens",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            A step-by-step guide to bridging tokens between Ethereum L1 and Mantle L2 using Shadow Swap.
          </p>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Prerequisites</h3>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Web3 wallet (MetaMask, WalletConnect, Coinbase Wallet, etc.)</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Tokens on source chain + gas fees</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Destination wallet address</span>
              </li>
            </ul>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Bridging Process</h3>
            <ol className="space-y-4">
              {[
                {
                  step: "Connect Wallet",
                  description:
                    'Visit the bridge page and click "Connect Wallet". Approve the connection in your wallet.',
                },
                {
                  step: "Select Networks",
                  description:
                    "Choose source network (where your tokens are) and destination network (where you want them).",
                },
                {
                  step: "Enter Amount",
                  description: 'Input the amount to bridge. Use "Max" to bridge your entire balance (minus gas fees).',
                },
                {
                  step: "Set Destination",
                  description:
                    "Enter destination address or use your connected wallet. This is where funds will arrive.",
                },
                {
                  step: "Review & Confirm",
                  description:
                    'Check the fee estimate and expected arrival time. Click "Bridge Now" and approve the transaction.',
                },
                {
                  step: "Track Progress",
                  description:
                    "Monitor the 5-step process: Intent creation → Solver matching → Fulfillment → Auto-claim → Complete.",
                },
              ].map((item, index) => (
                <li key={index} className="flex gap-4">
                  <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                    {index + 1}
                  </div>
                  <div>
                    <h4 className="mb-1 font-semibold text-white">{item.step}</h4>
                    <p className="text-sm text-neutral-400">{item.description}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">Tips for a Smooth Bridge</h3>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Always double-check the destination address before confirming</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Keep some tokens for gas fees on both chains</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Bridge during low congestion periods for faster settlement</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Save your transaction hash for tracking in Activity page</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
    "become-solver": {
      title: "Become a Solver",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Join the Shadow Swap solver network to earn fees by providing liquidity and fulfilling cross-chain intents.
          </p>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">Requirements</h3>
            <div className="space-y-3">
              <div>
                <h4 className="mb-2 font-medium text-white">Capital</h4>
                <p className="text-sm text-neutral-400">
                  Minimum 10 ETH recommended for consistent operation. More capital allows handling larger intents and
                  earning higher fees.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Technical Infrastructure</h4>
                <p className="text-sm text-neutral-400">
                  Reliable RPC endpoints for both Ethereum and Mantle, monitoring system, and automated rebalancing
                  logic.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Skills</h4>
                <p className="text-sm text-neutral-400">
                  Basic understanding of DeFi, smart contracts, and ability to run and maintain server infrastructure.
                </p>
              </div>
            </div>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Setup Guide</h3>
            <ol className="space-y-4">
              {[
                {
                  step: "Install Solver Software",
                  description: "Clone the repository and install dependencies: npm install",
                },
                {
                  step: "Configure Environment",
                  description:
                    "Set up RPC URLs, private key, and parameters in .env file. Never commit secrets to version control.",
                },
                {
                  step: "Fund Solver Wallet",
                  description: "Transfer liquidity to your solver wallet on both Ethereum and Mantle networks.",
                },
                {
                  step: "Start Monitoring",
                  description:
                    "Run the solver: npm run solver. It will monitor intents and automatically fill profitable ones.",
                },
                {
                  step: "Monitor Performance",
                  description:
                    "Track fills, success rate, and profitability. Adjust parameters based on market conditions.",
                },
              ].map((item, index) => (
                <li key={index} className="flex gap-4">
                  <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                    {index + 1}
                  </div>
                  <div>
                    <h4 className="mb-1 font-semibold text-white">{item.step}</h4>
                    <p className="text-sm text-neutral-400">{item.description}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">Risk Considerations</h3>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Smart contract risk: Audited but not formally verified</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Capital lockup: Funds may be temporarily locked during fills</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Gas price volatility: Spikes can reduce profitability</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Competition: More solvers = thinner margins</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
    "run-relayer": {
      title: "Run a Relayer",
      content: (
        <div className="space-y-6">
          <p className="text-lg text-neutral-300">
            Relayers synchronize state between chains and auto-claim funds for users. Learn how to run your own relayer
            node.
          </p>

          <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-6">
            <h3 className="mb-4 text-lg font-semibold text-white">What Does a Relayer Do?</h3>
            <div className="space-y-3">
              <div>
                <h4 className="mb-2 font-medium text-white">Root Synchronization</h4>
                <p className="text-sm text-neutral-400">
                  Monitors Merkle root updates on source chain and submits them to destination chain every 2 minutes.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Auto-Claiming</h4>
                <p className="text-sm text-neutral-400">
                  Watches for filled intents and automatically claims funds on behalf of users using pre-signed
                  authorizations.
                </p>
              </div>
              <div>
                <h4 className="mb-2 font-medium text-white">Privacy Enhancement</h4>
                <p className="text-sm text-neutral-400">
                  Batches operations to decorrelate user actions and adds timing noise for enhanced privacy.
                </p>
              </div>
            </div>
          </div>

          <div>
            <h3 className="mb-4 text-xl font-bold text-white">Setup Instructions</h3>
            <ol className="space-y-4">
              {[
                {
                  step: "System Requirements",
                  description: "2 vCPU, 4GB RAM, 50GB SSD. Rust 1.70+, PostgreSQL 14+, Redis 6+",
                },
                {
                  step: "Clone Repository",
                  description: "git clone https://github.com/mantle/shadow-swap && cd relayer",
                },
                {
                  step: "Configure Environment",
                  description:
                    "Copy .env.example to .env and set RPC URLs, private key, and database connection string.",
                },
                {
                  step: "Database Setup",
                  description: "Run migrations: cargo run --bin migrate && cargo run --bin seed",
                },
                {
                  step: "Start Relayer",
                  description: "cargo run --release. Monitor logs for successful root syncs and claims.",
                },
              ].map((item, index) => (
                <li key={index} className="flex gap-4">
                  <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-orange-500/10 font-bold text-orange-500">
                    {index + 1}
                  </div>
                  <div>
                    <h4 className="mb-1 font-semibold text-white">{item.step}</h4>
                    <p className="text-sm text-neutral-400">{item.description}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <div className="rounded-lg border border-orange-500/20 bg-orange-500/10 p-6">
            <h3 className="mb-2 text-lg font-semibold text-white">Relayer Economics</h3>
            <p className="mb-4 text-neutral-300">Relayers earn a small fee for each auto-claim transaction:</p>
            <ul className="space-y-2 text-neutral-300">
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Revenue: 0.01% of bridged volume + gas rebates</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Costs: Gas fees, infrastructure, monitoring tools</span>
              </li>
              <li className="flex items-start gap-2">
                <ChevronRight className="mt-1 h-4 w-4 text-orange-500" />
                <span>Break-even: ~$500K daily volume with current gas prices</span>
              </li>
            </ul>
          </div>
        </div>
      ),
    },
  }

  const currentContent = content[activeSection] || content["getting-started"]

  return (
    <div className="min-h-screen bg-black">
      <Navigation />

      {/* Main Content */}
      <main className="pt-16">
        <div className="mx-auto flex max-w-7xl gap-8 px-4 py-8 sm:px-6">
          {/* Sidebar Navigation - Desktop */}
          <aside className="hidden w-80 flex-shrink-0 lg:block">
            <div className="sticky top-24">
              <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-4">
                <div className="mb-6">
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform text-neutral-500" />
                    <input
                      type="text"
                      placeholder="Search docs..."
                      className="w-full rounded border border-neutral-700 bg-neutral-800 py-2 pl-10 pr-4 text-sm text-white focus:border-orange-500/50 focus:outline-none"
                    />
                  </div>
                </div>

                <ScrollArea className="h-[calc(100vh-200px)]">
                  <nav className="space-y-6">
                    {navigation.map((section, sectionIndex) => (
                      <div key={sectionIndex}>
                        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wider text-neutral-500">
                          {section.title}
                        </h3>
                        <ul className="space-y-1">
                          {section.items.map((item) => (
                            <li key={item.id}>
                              <button
                                onClick={() => setActiveSection(item.id)}
                                className={`flex w-full items-center gap-3 rounded px-3 py-2 text-sm transition-colors ${
                                  activeSection === item.id
                                    ? "bg-orange-500/10 font-medium text-orange-500"
                                    : "text-neutral-400 hover:bg-neutral-800 hover:text-white"
                                }`}
                              >
                                <item.icon className="h-4 w-4 flex-shrink-0" />
                                <span>{item.title}</span>
                              </button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    ))}
                  </nav>
                </ScrollArea>

                {/* Help Links */}
                <div className="mt-6 space-y-2 border-t border-neutral-800 pt-6">
                  <a
                    href="https://github.com/Mist-Labs/Mantle-Protocol"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-2 text-sm text-neutral-400 transition-colors hover:text-orange-500"
                  >
                    <Github className="h-4 w-4" />
                    GitHub
                  </a>
                  <a
                    href="#"
                    className="flex items-center gap-2 text-sm text-neutral-400 transition-colors hover:text-orange-500"
                  >
                    <MessageCircle className="h-4 w-4" />
                    Discord
                  </a>
                </div>
              </div>
            </div>
          </aside>

          {/* Main Content Area */}
          <div className="min-w-0 flex-1">
            <article className="max-w-3xl">
              <div className="mb-6">
                <Badge variant="outline" className="mb-3 border-orange-500/50 text-orange-500">
                  DOCUMENTATION
                </Badge>
                <h1 className="mb-4 text-4xl font-bold text-white">{currentContent.title}</h1>
              </div>

              <div className="prose prose-invert max-w-none">{currentContent.content}</div>

              {/* Page Navigation */}
              <div className="mt-12 flex items-center justify-between border-t border-neutral-800 pt-8">
                <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800">
                  ← Previous
                </Button>
                <Button variant="outline" className="border-neutral-700 bg-neutral-900 hover:bg-neutral-800">
                  Next →
                </Button>
              </div>
            </article>
          </div>
        </div>
      </main>
    </div>
  )
}
