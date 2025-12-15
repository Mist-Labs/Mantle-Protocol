"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { Menu, X } from "lucide-react"
import { Button } from "@/components/ui/button"

export default function Navigation() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const pathname = usePathname()

  const navItems = [
    { name: "Bridge", href: "/bridge" },
    { name: "Activity", href: "/activity" },
    { name: "Stats", href: "/stats" },
    { name: "Docs", href: "/docs" },
  ]

  const isActive = (href: string) => pathname === href

  return (
    <header className="fixed top-0 left-0 right-0 z-50 h-16 bg-neutral-900/80 border-b border-neutral-800 backdrop-blur-xl">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 h-full flex items-center justify-between">
        <div className="flex items-center gap-8">
          <Link href="/" className="flex items-center gap-2">
            <div className="w-8 h-8 bg-gradient-to-br from-orange-500 to-pink-500 rounded rotate-45"></div>
            <h1 className="text-orange-500 font-bold text-lg tracking-wider">SHADOW SWAP</h1>
          </Link>

          {/* Desktop Navigation */}
          <nav className="hidden md:flex items-center gap-6">
            {navItems.map((item) => (
              <Link
                key={item.name}
                href={item.href}
                className={`text-sm transition-colors ${
                  isActive(item.href)
                    ? "text-orange-500 font-semibold"
                    : "text-neutral-400 hover:text-white"
                }`}
              >
                {item.name.toUpperCase()}
              </Link>
            ))}
          </nav>
        </div>

        <div className="flex items-center gap-3">
          {/* Network Badge */}
          <div className="hidden sm:flex items-center gap-2 bg-neutral-800 border border-neutral-700 px-3 py-1.5 rounded text-xs">
            <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
            <span className="text-white">MAINNET</span>
          </div>

          {/* Connect Wallet Button */}
          <Button className="bg-orange-500 hover:bg-orange-600 text-white shadow-lg shadow-orange-500/20 transition-all duration-300 hover:scale-105">
            Connect Wallet
          </Button>

          {/* Mobile Menu */}
          <button
            className="md:hidden text-neutral-400 hover:text-white"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
          </button>
        </div>
      </div>

      {/* Mobile Menu Dropdown */}
      {mobileMenuOpen && (
        <div className="md:hidden absolute top-16 left-0 right-0 bg-neutral-900 border-b border-neutral-800 shadow-2xl">
          <nav className="flex flex-col p-4 gap-2">
            {navItems.map((item) => (
              <Link
                key={item.name}
                href={item.href}
                onClick={() => setMobileMenuOpen(false)}
                className={`text-left py-2 px-3 rounded transition-colors ${
                  isActive(item.href)
                    ? "bg-orange-500 text-white"
                    : "text-neutral-400 hover:text-white hover:bg-neutral-800"
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
