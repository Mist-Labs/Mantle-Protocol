/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  // Turbopack configuration for Next.js 16+
  // Empty config to silence webpack/turbopack migration warning
  turbopack: {},
  // External packages that should not be bundled
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
  // Webpack configuration (fallback for when using --webpack flag)
  webpack: (config, { webpack, isServer }) => {
    if (!isServer) {
      // Client-side polyfills
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
        crypto: false,
      }
    }

    // Ignore test dependencies
    config.plugins.push(
      new webpack.IgnorePlugin({
        resourceRegExp: /^(tap|desm|fastbench|pino-elasticsearch|why-is-node-running)$/,
      })
    )

    return config
  },
}

export default nextConfig
