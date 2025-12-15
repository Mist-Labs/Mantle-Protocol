"use client"

import Link from "next/link"
import { Twitter, Github, MessageCircle } from "lucide-react"

export default function Footer() {
  return (
    <footer className="border-t border-neutral-800 bg-neutral-950 px-4 py-12 sm:px-6">
      <div className="mx-auto max-w-7xl">
        <div className="mb-8 grid grid-cols-1 gap-8 md:grid-cols-4">
          {/* Brand */}
          <div className="col-span-1 md:col-span-2">
            <div className="mb-4 flex items-center gap-2">
              <div className="h-8 w-8 rotate-45 rounded bg-gradient-to-br from-orange-500 to-pink-500"></div>
              <h3 className="text-lg font-bold tracking-wider text-orange-500">SHADOW SWAP</h3>
            </div>
            <p className="mb-4 text-sm text-neutral-400">
              Privacy-preserving cross-chain bridge built on Mantle L2. Fast, secure, and truly private.
            </p>
            <div className="flex gap-3">
              <a
                href="#"
                className="flex h-9 w-9 items-center justify-center rounded border border-neutral-800 bg-neutral-900 transition-colors hover:border-orange-500/50"
              >
                <Twitter className="h-4 w-4 text-neutral-400" />
              </a>
              <a
                href="https://github.com/Mist-Labs/Mantle-Protocol"
                target="_blank"
                rel="noopener noreferrer"
                className="flex h-9 w-9 items-center justify-center rounded border border-neutral-800 bg-neutral-900 transition-colors hover:border-orange-500/50"
              >
                <Github className="h-4 w-4 text-neutral-400" />
              </a>
              <a
                href="#"
                className="flex h-9 w-9 items-center justify-center rounded border border-neutral-800 bg-neutral-900 transition-colors hover:border-orange-500/50"
              >
                <MessageCircle className="h-4 w-4 text-neutral-400" />
              </a>
            </div>
          </div>

          {/* Quick Links */}
          <div>
            <h4 className="mb-4 font-semibold text-white">Product</h4>
            <ul className="space-y-2">
              <li>
                <Link href="/bridge" className="text-sm text-neutral-400 transition-colors hover:text-orange-500">
                  Bridge
                </Link>
              </li>
              <li>
                <Link href="/activity" className="text-sm text-neutral-400 transition-colors hover:text-orange-500">
                  Activity
                </Link>
              </li>
              <li>
                <Link href="/stats" className="text-sm text-neutral-400 transition-colors hover:text-orange-500">
                  Stats
                </Link>
              </li>
            </ul>
          </div>

          {/* Resources */}
          <div>
            <h4 className="mb-4 font-semibold text-white">Resources</h4>
            <ul className="space-y-2">
              <li>
                <Link href="/docs" className="text-sm text-neutral-400 transition-colors hover:text-orange-500">
                  Documentation
                </Link>
              </li>
              <li>
                <a
                  href="https://github.com/Mist-Labs/Mantle-Protocol"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-neutral-400 transition-colors hover:text-orange-500"
                >
                  GitHub
                </a>
              </li>
              <li>
                <a href="#" className="text-sm text-neutral-400 transition-colors hover:text-orange-500">
                  Support
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom Bar */}
        <div className="flex flex-col items-center justify-between gap-4 border-t border-neutral-800 pt-8 sm:flex-row">
          <p className="text-sm text-neutral-500">
            © 2025 Shadow Swap. Built on <span className="text-orange-500">Mantle</span>.
          </p>
          <div className="flex items-center gap-2 text-xs text-neutral-500">
            <div className="h-2 w-2 animate-pulse rounded-full bg-green-500"></div>
            <span>All systems operational</span>
          </div>
        </div>
      </div>
    </footer>
  )
}
