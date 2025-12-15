/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  serverExternalPackages: [
    "pino",
    "thread-stream",
    "@walletconnect/logger",
    "@walletconnect/ethereum-provider",
    "@walletconnect/universal-provider",
    "@walletconnect/sign-client",
    "@coinbase/wallet-sdk",
    "@metamask/sdk",
    "pino-pretty",
    "lokijs",
    "encoding",
  ],
}

export default nextConfig
