# Shadow Swap Frontend

Privacy-preserving cross-chain bridge interface built with Next.js, React, and TailwindCSS.

## 🚀 Quick Start

```bash
# Install dependencies
make install

# Run development server
make dev

# Build for production
make build

# Start production server
make start

# Lint code
make lint

# Clean build artifacts
make clean
```

## 📦 Tech Stack

- **Framework:** Next.js 16.0.10 (App Router)
- **Language:** TypeScript
- **Styling:** TailwindCSS + shadcn/ui
- **Web3:** Reown AppKit (formerly WalletConnect) + Wagmi + Viem
- **Charts:** Recharts
- **Animations:** Framer Motion
- **Forms:** React Hook Form + Zod validation
- **State Management:** React Query

## 🏗️ Project Structure

```
packages/frontend/
├── app/                      # Next.js app directory
│   ├── (landing)/           # Landing page route group
│   ├── bridge/              # Bridge interface
│   ├── activity/            # Transaction history
│   ├── stats/               # Analytics dashboard
│   ├── docs/                # Documentation
│   ├── layout.tsx           # Root layout
│   └── globals.css          # Global styles
├── components/
│   ├── landing/             # Landing page components
│   ├── bridge/              # Bridge page components
│   ├── activity/            # Activity page components
│   ├── stats/               # Stats page components
│   ├── docs/                # Docs page components
│   ├── shared/              # Shared components (Nav, Footer)
│   └── ui/                  # shadcn/ui components
├── lib/                     # Utilities and helpers
├── hooks/                   # Custom React hooks
├── public/                  # Static assets
└── styles/                  # Additional styles
```

## 🎨 Features

- **Landing Page:** Professional hero with animated stats, features, and how-it-works sections
- **Bridge Interface:** Interactive bridge form with network selection, amount input, and wallet integration
- **Activity Dashboard:** Transaction history with filters, search, and detail modals
- **Stats Analytics:** Real-time charts and metrics for protocol performance
- **Documentation:** Comprehensive docs with sidebar navigation
- **Responsive Design:** Mobile-first, works on all devices
- **Dark Theme:** Cyberpunk aesthetic with orange accents
- **Web3 Integration:** Wallet connection via Reown AppKit

## 🔧 Configuration

### Environment Variables

Create a `.env.local` file:

```env
# Reown AppKit
NEXT_PUBLIC_REOWN_PROJECT_ID=your_project_id_here

# RPC URLs
NEXT_PUBLIC_MANTLE_RPC_URL=https://rpc.mantle.xyz
NEXT_PUBLIC_ETHEREUM_RPC_URL=https://eth.llamarpc.com

# Contract Addresses
NEXT_PUBLIC_MANTLE_INTENT_POOL=0x...
NEXT_PUBLIC_MANTLE_SETTLEMENT=0x...
NEXT_PUBLIC_ETHEREUM_SETTLEMENT=0x...

# API
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080

# Explorer URLs
NEXT_PUBLIC_MANTLE_EXPLORER=https://explorer.mantle.xyz
NEXT_PUBLIC_ETHEREUM_EXPLORER=https://etherscan.io
```

### Wallet Configuration

Get your Reown Project ID at: https://cloud.reown.com

## 📝 Development

### Commands

- `make dev` - Start development server (http://localhost:3000)
- `make build` - Build for production
- `make start` - Start production server
- `make lint` - Run ESLint
- `make clean` - Remove build artifacts and dependencies
- `make install` - Install all dependencies

### Code Style

- TypeScript with strict mode
- ESLint + Prettier for code formatting
- Conventional commits for Git messages

## 🧪 Testing

```bash
# Run unit tests
npm run test

# Run integration tests
npm run test:integration

# Run E2E tests
npm run test:e2e
```

## 🚢 Deployment

### Vercel (Recommended)

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
vercel
```

### Docker

```bash
# Build image
docker build -t shadow-swap-frontend .

# Run container
docker run -p 3000:3000 shadow-swap-frontend
```

## 📚 Documentation

- [Next.js Documentation](https://nextjs.org/docs)
- [TailwindCSS](https://tailwindcss.com/docs)
- [shadcn/ui](https://ui.shadcn.com)
- [Reown AppKit](https://docs.reown.com/appkit/overview)
- [Wagmi](https://wagmi.sh)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License.

## 🔗 Links

- [GitHub](https://github.com/Mist-Labs/Mantle-Protocol)
- [Documentation](https://docs.shadowswap.xyz)
- [Discord](https://discord.gg/shadowswap)
- [Twitter](https://twitter.com/shadowswap)

---

Built with ❤️ for the Mantle ecosystem
