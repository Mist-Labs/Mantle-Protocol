import { createAppKit } from "@reown/appkit/react"
import { WagmiAdapter } from "@reown/appkit-adapter-wagmi"
import { sepolia } from "@reown/appkit/networks"
import { QueryClient } from "@tanstack/react-query"
import { defineChain } from "viem"

// Get projectId from environment
const projectId = process.env.NEXT_PUBLIC_REOWN_PROJECT_ID || ""

if (!projectId) {
  console.warn("NEXT_PUBLIC_REOWN_PROJECT_ID is not set. Wallet connection will not work.")
}

// Define Mantle Sepolia with correct configuration
const mantleSepoliaTestnet = defineChain({
  id: 5003,
  name: "Mantle Sepolia Testnet",
  nativeCurrency: {
    name: "MNT",
    symbol: "MNT",
    decimals: 18,
  },
  rpcUrls: {
    default: {
      http: ["https://rpc.sepolia.mantle.xyz"],
    },
    public: {
      http: ["https://rpc.sepolia.mantle.xyz"],
    },
  },
  blockExplorers: {
    default: {
      name: "Mantle Sepolia Explorer",
      url: "https://explorer.sepolia.mantle.xyz",
    },
  },
  testnet: true,
})

// Configure supported testnets only
export const networks = [mantleSepoliaTestnet, sepolia]

// Metadata for the app
const metadata = {
  name: "Shadow Swap",
  description: "Privacy-preserving cross-chain bridge for Mantle L2",
  url: typeof window !== "undefined" ? window.location.origin : "https://shadowswap.xyz",
  icons: ["/icon.svg"],
}

// Create QueryClient for Wagmi
// Note: Query-specific config is handled per-query, not globally
export const queryClient = new QueryClient()

// Create Wagmi Adapter
export const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId,
  ssr: true,
})

// Create AppKit instance
export const modal = createAppKit({
  adapters: [wagmiAdapter],
  networks: [mantleSepoliaTestnet, sepolia],
  projectId,
  metadata,
  features: {
    analytics: false,
    email: false,
    socials: [],
  },
  themeMode: "dark",
  themeVariables: {
    "--w3m-accent": "#f97316", // Orange-500
    "--w3m-border-radius-master": "8px",
  },
})

// Export wagmi config
export const config = wagmiAdapter.wagmiConfig
