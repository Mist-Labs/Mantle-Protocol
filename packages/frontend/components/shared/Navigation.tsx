"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { Menu, X, ChevronDown } from "lucide-react"
import { useAccount, useChainId, useSwitchChain } from "wagmi"
import { useAppKit } from "@reown/appkit/react"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { toast } from "sonner"

export default function Navigation() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const pathname = usePathname()
  const { address, isConnected } = useAccount()
  const chainId = useChainId()
  const { open } = useAppKit()
  const { switchChain } = useSwitchChain()

  const navItems = [
    { name: "Bridge", href: "/bridge" },
    { name: "Activity", href: "/activity" },
    { name: "Stats", href: "/stats" },
    { name: "Docs", href: "/docs" },
  ]

  const isActive = (href: string) => pathname === href

  // Available networks
  const networks = [
    { id: 5000, name: "Mantle Mainnet", value: "5000" },
    { id: 5003, name: "Mantle Sepolia", value: "5003" },
    { id: 1, name: "Ethereum Mainnet", value: "1" },
    { id: 11155111, name: "Ethereum Sepolia", value: "11155111" },
  ]

  // Handle network switch
  const handleNetworkSwitch = (value: string) => {
    const targetChainId = parseInt(value)
    if (!isConnected) {
      toast.error("Please connect your wallet first")
      return
    }
    if (switchChain) {
      switchChain(
        { chainId: targetChainId },
        {
          onSuccess: () => {
            toast.success(`Switched to ${networks.find((n) => n.id === targetChainId)?.name}`)
          },
          onError: (error) => {
            toast.error(`Failed to switch network: ${error.message}`)
          },
        }
      )
    }
  }

  // Get current network name
  const getCurrentNetwork = () => {
    return networks.find((n) => n.id === chainId)
  }

  return (
    <header className="fixed left-0 right-0 top-0 z-50 h-16 border-b border-neutral-800 bg-neutral-900/80 backdrop-blur-xl">
      <div className="mx-auto flex h-full max-w-7xl items-center justify-between px-4 sm:px-6">
        <div className="flex items-center gap-8">
          <Link href="/" className="flex items-center gap-2">
            <div className="h-8 w-8 rotate-45 rounded bg-gradient-to-br from-orange-500 to-pink-500"></div>
            <h1 className="text-lg font-bold tracking-wider text-orange-500">SHADOW SWAP</h1>
          </Link>

          {/* Desktop Navigation */}
          <nav className="hidden items-center gap-6 md:flex">
            {navItems.map((item) => (
              <Link
                key={item.name}
                href={item.href}
                className={`text-sm transition-colors ${
                  isActive(item.href) ? "font-semibold text-orange-500" : "text-neutral-400 hover:text-white"
                }`}
              >
                {item.name.toUpperCase()}
              </Link>
            ))}
          </nav>
        </div>

        <div className="flex items-center gap-3">
          {/* Network Selector */}
          {isConnected && (
            <Select value={chainId?.toString()} onValueChange={handleNetworkSwitch}>
              <SelectTrigger className="hidden w-[220px] border-neutral-700 bg-neutral-800 text-white sm:flex">
                <div className="flex items-center gap-2">
                  <div className="h-2 w-2 animate-pulse rounded-full bg-green-500"></div>
                  <SelectValue placeholder="Select Network">
                    {getCurrentNetwork()?.name || "Unknown Network"}
                  </SelectValue>
                </div>
              </SelectTrigger>
              <SelectContent>
                {networks.map((network) => (
                  <SelectItem key={network.id} value={network.value}>
                    {network.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}

          {/* Connect Wallet Button */}
          <Button
            onClick={() => open()}
            className="bg-orange-500 text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105 hover:bg-orange-600"
          >
            {isConnected ? `${address?.slice(0, 6)}...${address?.slice(-4)}` : "Connect Wallet"}
          </Button>

          {/* Mobile Menu */}
          <button
            className="text-neutral-400 hover:text-white md:hidden"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            {mobileMenuOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
          </button>
        </div>
      </div>

      {/* Mobile Menu Dropdown */}
      {mobileMenuOpen && (
        <div className="absolute left-0 right-0 top-16 border-b border-neutral-800 bg-neutral-900 shadow-2xl md:hidden">
          <nav className="flex flex-col gap-2 p-4">
            {navItems.map((item) => (
              <Link
                key={item.name}
                href={item.href}
                onClick={() => setMobileMenuOpen(false)}
                className={`rounded px-3 py-2 text-left transition-colors ${
                  isActive(item.href)
                    ? "bg-orange-500 text-white"
                    : "text-neutral-400 hover:bg-neutral-800 hover:text-white"
                }`}
              >
                {item.name.toUpperCase()}
              </Link>
            ))}
          </nav>
        </div>
      )}
    </header>
  )
}
