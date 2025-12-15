"use client"

import Link from "next/link"
import { Twitter, Github, MessageCircle } from "lucide-react"

export default function Footer() {
  return (
    <footer className="border-t border-neutral-800 bg-neutral-950 py-12 px-4 sm:px-6">
      <div className="max-w-7xl mx-auto">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8 mb-8">
          {/* Brand */}
          <div className="col-span-1 md:col-span-2">
            <div className="flex items-center gap-2 mb-4">
              <div className="w-8 h-8 bg-gradient-to-br from-orange-500 to-pink-500 rounded rotate-45"></div>
              <h3 className="text-orange-500 font-bold text-lg tracking-wider">SHADOW SWAP</h3>
            </div>
            <p className="text-neutral-400 text-sm mb-4">
              Privacy-preserving cross-chain bridge built on Mantle L2. Fast, secure, and truly private.
            </p>
            <div className="flex gap-3">
              <a
                href="#"
                className="w-9 h-9 bg-neutral-900 border border-neutral-800 rounded flex items-center justify-center hover:border-orange-500/50 transition-colors"
              >
                <Twitter className="w-4 h-4 text-neutral-400" />
              </a>
              <a
                href="#"
                className="w-9 h-9 bg-neutral-900 border border-neutral-800 rounded flex items-center justify-center hover:border-orange-500/50 transition-colors"
              >
                <Github className="w-4 h-4 text-neutral-400" />
              </a>
              <a
                href="#"
                className="w-9 h-9 bg-neutral-900 border border-neutral-800 rounded flex items-center justify-center hover:border-orange-500/50 transition-colors"
              >
                <MessageCircle className="w-4 h-4 text-neutral-400" />
              </a>
            </div>
          </div>

          {/* Quick Links */}
          <div>
            <h4 className="text-white font-semibold mb-4">Product</h4>
            <ul className="space-y-2">
              <li>
                <Link href="/bridge" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  Bridge
                </Link>
              </li>
              <li>
                <Link href="/activity" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  Activity
                </Link>
              </li>
              <li>
                <Link href="/stats" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  Stats
                </Link>
              </li>
            </ul>
          </div>

          {/* Resources */}
          <div>
            <h4 className="text-white font-semibold mb-4">Resources</h4>
            <ul className="space-y-2">
              <li>
                <Link href="/docs" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  Documentation
                </Link>
              </li>
              <li>
                <a href="#" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  GitHub
                </a>
              </li>
              <li>
                <a href="#" className="text-neutral-400 hover:text-orange-500 text-sm transition-colors">
                  Support
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom Bar */}
        <div className="pt-8 border-t border-neutral-800 flex flex-col sm:flex-row justify-between items-center gap-4">
          <p className="text-neutral-500 text-sm">
            © 2025 Shadow Swap. Built on <span className="text-orange-500">Mantle</span>.
          </p>
          <div className="flex items-center gap-2 text-xs text-neutral-500">
            <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
            <span>All systems operational</span>
          </div>
        </div>
      </div>
    </footer>
  )
}
