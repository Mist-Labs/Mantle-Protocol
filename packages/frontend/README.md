# Shadow Swap Frontend

Privacy-preserving cross-chain bridge interface built with Next.js, React, and TailwindCSS.

**Live Demo:** [https://shadow-swaps.vercel.app/](https://shadow-swaps.vercel.app/)

## 🚀 Quick Start

**IMPORTANT:** This is part of a monorepo. All `make` commands must be run from the **repository root**, not from this directory.

```bash
# From repository root (Mantle-Protocol/)

# Install frontend dependencies
make frontend-install

# Run frontend development server
make frontend-dev

# Build frontend for production
make frontend-build

# Start frontend production server
make frontend-start

# Lint frontend code
make frontend-lint

# Type check frontend
make frontend-typecheck

# Clean frontend build artifacts
make frontend-clean
```

### Alternative: NPM Commands (from this directory)

If you prefer to work directly in the frontend package:

```bash
# From packages/frontend/
npm install
npm run dev          # Start dev server
npm run build        # Build for production
npm run start        # Start production server
npm run lint         # Run ESLint
npm run typecheck    # Run TypeScript checks
```

## 📑 Table of Contents

- [Tech Stack](#-tech-stack)
- [Project Structure](#-project-structure)
- [Features](#-features)
- [Application Pages](#-application-pages)
- [Configuration](#-configuration)
- [Privacy & Security Model](#-privacy--security-model)
- [API Integration](#-api-integration)
- [Development](#-development)
- [Deployment](#-deployment)
- [Troubleshooting](#-troubleshooting)
- [Supported Networks](#-supported-networks)
- [Architecture](#-architecture)
- [Contributing](#-contributing)

## 📦 Tech Stack

- **Framework:** Next.js 16.0.10 (App Router)
- **Runtime:** React 19.2.3
- **Language:** TypeScript 5
- **Styling:** TailwindCSS 3.4 + shadcn/ui
- **Web3:** Reown AppKit 1.8 (formerly WalletConnect) + Wagmi 3.1 + Viem 2.42
- **Charts:** Recharts 2.15
- **Animations:** Framer Motion 12
- **Forms:** React Hook Form 7.60 + Zod 3.25
- **State Management:** TanStack Query 5.90
- **Analytics:** Vercel Analytics
- **Node Version:** >= 20.0.0

## 🏗️ Project Structure

### Monorepo Context

This is the **frontend package** within the Shadow Swap monorepo:

```
Mantle-Protocol/                 # Repository root
├── packages/
│   ├── frontend/                # ← This package (Next.js dApp)
│   ├── shadow-swap/             # Rust relayer backend
│   ├── contracts/               # Solidity smart contracts (Foundry)
│   ├── Indexers/                # Event indexer (Node.js)
│   └── solver/                  # Solver bot (Rust)
├── Makefile                     # Root commands for all packages
└── README.md                    # Main project documentation
```

### Frontend Package Structure

```
packages/frontend/
├── app/                      # Next.js app directory (App Router)
│   ├── (landing)/           # Landing page route group
│   ├── bridge/              # Bridge interface (/bridge)
│   ├── activity/            # Transaction history (/activity)
│   ├── stats/               # Analytics dashboard (/stats)
│   ├── docs/                # Documentation (/docs)
│   ├── layout.tsx           # Root layout with providers
│   ├── loading.tsx          # Loading states
│   └── globals.css          # Global styles + Tailwind
├── components/
│   ├── landing/             # Landing page components
│   ├── bridge/              # Bridge form, progress, recent activity
│   ├── activity/            # Activity table and filters
│   ├── stats/               # Charts and metrics
│   ├── docs/                # Documentation components
│   ├── shared/              # Navigation, Footer, Web3Provider
│   └── ui/                  # shadcn/ui component library (60+ components)
├── lib/                     # Utilities (crypto, contracts, config)
├── hooks/                   # Custom React hooks (Web3, API calls)
├── scripts/                 # Build and deployment scripts
├── public/                  # Static assets (images, icons, fonts)
├── styles/                  # Additional CSS modules
└── .env.example             # Environment configuration template
```

## 🎨 Features

### Core Functionality
- **Privacy-Preserving Bridge:** Commitment-based architecture for private cross-chain transfers
- **Intent-Based Settlement:** ERC-7683 compatible with solver network integration
- **Fast Bridging:** 10-30 second transfers between Ethereum Sepolia ↔ Mantle Sepolia
- **Low Fees:** Competitive 0.15% total fee structure

### User Interface
- **Landing Page:** Professional hero with animated stats, features showcase, and how-it-works workflow
- **Bridge Interface:** Interactive form with network/token selection, amount input, and real-time fee calculation
- **Activity Dashboard:** Transaction history with status tracking, filters, and detailed transaction views
- **Stats Analytics:** Real-time charts and metrics for protocol performance (volume, TVL, user stats)
- **Documentation:** Comprehensive in-app documentation with guides and technical details

### Technical Features
- **Multi-Wallet Support:** MetaMask, WalletConnect, Coinbase Wallet via Reown AppKit
- **Responsive Design:** Mobile-first, optimized for all devices
- **Dark Theme:** Cyberpunk aesthetic with orange/yellow accent colors
- **ECIES Encryption:** Secure secret encryption before transmission to relayer
- **Real-Time Updates:** Live transaction status monitoring and notifications

## 📱 Application Pages

| Route | Description | Key Components |
|-------|-------------|----------------|
| `/` | Landing page with hero, features, how-it-works sections | HeroSection, FeaturesSection, HowItWorksSection |
| `/bridge` | Main bridge interface for cross-chain transfers | BridgeForm, BridgeProgress, RecentActivity |
| `/activity` | Transaction history and status tracking | Activity table with filters and search |
| `/stats` | Protocol analytics and metrics dashboard | Charts for volume, TVL, transactions, users |
| `/docs` | In-app documentation and guides | Documentation with sidebar navigation |

## 🔧 Configuration

### Environment Variables

A comprehensive `.env.example` file is provided. Copy it to create your local configuration:

```bash
cp .env.example .env.local
```

**Key Variables:**

```env
# Wallet Connection (Required)
NEXT_PUBLIC_REOWN_PROJECT_ID=your_project_id_here

# Backend API
NEXT_PUBLIC_API_BASE_URL=https://international-linnie-mist-labs-2c5cd590.koyeb.app/api/v1
NEXT_PUBLIC_HMAC_SECRET=your_hmac_secret_here

# Cryptography
NEXT_PUBLIC_RELAYER_PUBLIC_KEY=044c8cc1938e538d55209f04dd29a785a95391f7e00aac9385e45f38bf33ea5f4e...

# Ethereum Sepolia Testnet (Chain ID: 11155111)
NEXT_PUBLIC_ETHEREUM_INTENT_POOL=0xcb46d916522d7c6853fce2aa5f337e0a3626e263
NEXT_PUBLIC_ETHEREUM_SETTLEMENT=0x7CCC9864125143e6c530506772Eaf5595DC14897
NEXT_PUBLIC_ETHEREUM_POSEIDON_HASHER=0x5d3efc46ddba9f083b571a64210B976E06e6d7B2
NEXT_PUBLIC_ETHEREUM_RPC_URL=https://rpc.sepolia.org
NEXT_PUBLIC_ETHEREUM_EXPLORER=https://sepolia.etherscan.io

# Mantle Sepolia Testnet (Chain ID: 5003)
NEXT_PUBLIC_MANTLE_INTENT_POOL=0x6ebcf830b855108fa44abed6ba964f2af9c34424
NEXT_PUBLIC_MANTLE_SETTLEMENT=0x1c4F9eBeccE31cEFe2FDe415b05184b4ea46908f
NEXT_PUBLIC_MANTLE_POSEIDON_HASHER=0x8EA86eD4317AF92f73E5700eB9b93A72dE62f3B1
NEXT_PUBLIC_MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
NEXT_PUBLIC_MANTLE_EXPLORER=https://sepolia.mantlescan.xyz/
```

**Setup Steps:**
1. Get your Reown Project ID from [cloud.reown.com](https://cloud.reown.com)
2. Get HMAC secret from the backend team (for production, use a Backend-for-Frontend pattern)
3. Update `.env.local` with your values
4. Never commit `.env.local` to version control

**Supported Tokens:**

*Ethereum Sepolia:*
- ETH (Native): `0x0000000000000000000000000000000000000000`
- USDC: `0x28650373758d75a8fF0B22587F111e47BAC34e21`
- USDT: `0x89F4f0e13997Ca27cEB963DEE291C607e4E59923`
- WETH: `0x50e8Da97BeEB8064714dE45ce1F250879f3bD5B5`
- MNT: `0x65e37B558F64e2Be5768DB46DF22F93d85741A9E`

*Mantle Sepolia:*
- ETH (Native): `0x0000000000000000000000000000000000000000`
- USDC: `0xA4b184006B59861f80521649b14E4E8A72499A23`
- USDT: `0xB0ee6EF7788E9122fc4AAE327Ed4FEf56c7da891`
- WETH: `0xdeaddeaddeaddeaddeaddeaddeaddeaddead1111`
- MNT: `0x44FCE297e4D6c5A50D28Fb26A58202e4D49a13E7`

See `.env.example` for complete configuration details.

## 🔐 Privacy & Security Model

### Commitment-Based Privacy

Shadow Swap uses a commitment scheme to decouple senders from receivers:

```
secret (random 32 bytes)
   ↓
nullifier = keccak256(secret)
   ↓
commitment = Poseidon(secret, nullifier, amount, sourceChain)
```

- **On-chain:** Only commitments are stored (no link between depositor and withdrawer)
- **Off-chain:** Secrets are ECIES-encrypted and sent to relayer
- **Settlement:** Relayer uses pre-signed authorization for automated claiming

### ECIES Encryption

All sensitive data (secret, nullifier) is encrypted before transmission:

1. Generate random secret and derive nullifier
2. Compute Poseidon commitment on-chain
3. Sign claim authorization for relayer
4. ECIES encrypt secret/nullifier with relayer's public key
5. Send encrypted data to API after on-chain transaction succeeds

### Bridge Flow

1. **User:** Generate privacy parameters (secret, nullifier, commitment)
2. **User:** Submit on-chain transaction with commitment
3. **User → API:** Send encrypted parameters after tx confirmation
4. **Relayer:** Monitor events, build merkle proofs, register intents
5. **Solver:** Provide liquidity on destination chain
6. **Relayer:** Automatically claim funds using pre-signed authorization
7. **Complete:** User receives funds on destination chain

### Fee Structure

- **Bridge Fee:** 0.2% (20 basis points) deducted from bridged amount
- **No gas on destination:** Relayer handles all destination chain transactions

## 🌐 API Integration

The frontend integrates with the backend relayer API for:

### Available Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/bridge/initiate` | POST | Yes | Submit encrypted privacy parameters |
| `/bridge/intent/{id}` | GET | No | Check intent status |
| `/bridge/intents` | GET | No | List intents with filters |
| `/price` | GET | No | Get token exchange rates |
| `/prices/all` | GET | No | Get all USD prices |
| `/health` | GET | No | System health check |
| `/metrics` | GET | No | Operational metrics |
| `/stats` | GET | No | Bridge statistics |

### HMAC Authentication

Protected endpoints require HMAC-SHA256 authentication:

```typescript
// Generate signature
const timestamp = Math.floor(Date.now() / 1000).toString();
const message = timestamp + JSON.stringify(requestBody);
const signature = crypto
  .createHmac('sha256', HMAC_SECRET)
  .update(message)
  .digest('hex');

// Send with headers
headers: {
  'Content-Type': 'application/json',
  'X-Signature': signature,
  'X-Timestamp': timestamp
}
```

### Intent Status Lifecycle

| Status | Description |
|--------|-------------|
| `committed` | Intent created, tokens escrowed on source chain |
| `registered` | Intent registered on destination chain (ready for filling) |
| `filled` | Solver provided liquidity on destination chain |
| `solver_paid` | Solver claimed escrowed funds on source chain |
| `user_claimed` | User claimed funds on destination (complete) |
| `refunded` | Intent refunded to user |
| `cancelled` | Intent cancelled by user |

## 📝 Development

### Available Commands (from repo root)

| Command | Description |
|---------|-------------|
| `make frontend-install` | Install dependencies |
| `make frontend-dev` | Start dev server at http://localhost:3000 |
| `make frontend-build` | Build for production |
| `make frontend-start` | Start production server |
| `make frontend-lint` | Run ESLint |
| `make frontend-typecheck` | Run TypeScript type checking |
| `make frontend-clean` | Remove .next, node_modules, and out directories |

### Direct NPM Scripts (from packages/frontend/)

| Script | Command |
|--------|---------|
| `npm run dev` | Start development server |
| `npm run build` | Build for production |
| `npm run start` | Start production server |
| `npm run lint` | Run ESLint |
| `npm run lint:fix` | Run ESLint with auto-fix |
| `npm run format` | Format code with Prettier |
| `npm run format:check` | Check code formatting |
| `npm run typecheck` | Run TypeScript type checking |

### Code Standards

- **TypeScript:** Strict mode enabled
- **Linting:** ESLint with Next.js configuration
- **Formatting:** Prettier with Tailwind CSS plugin
- **Commits:** Conventional commit format recommended
- **Node Version:** >= 20.0.0 (check with `node --version`)

### Development Workflow

1. Ensure Node.js >= 20.0.0 is installed
2. Install dependencies: `make frontend-install` or `npm install`
3. Copy `.env.example` to `.env.local` and configure
4. Start dev server: `make frontend-dev` or `npm run dev`
5. Open [http://localhost:3000](http://localhost:3000) in your browser

### Key Libraries & Utilities

**Web3 Integration:**
- `@reown/appkit` - Multi-wallet connection interface
- `wagmi` - React hooks for Ethereum
- `viem` - TypeScript Ethereum library
- `eciesjs` - ECIES encryption for privacy parameters

**UI Components:**
- `@radix-ui/*` - Unstyled, accessible component primitives
- `shadcn/ui` - Pre-built components with Radix + Tailwind
- `lucide-react` - Icon library
- `framer-motion` - Animation library

**Forms & Validation:**
- `react-hook-form` - Form state management
- `zod` - Schema validation
- `@hookform/resolvers` - Zod integration with React Hook Form

**Data Fetching:**
- `@tanstack/react-query` - Async state management
- Built-in `fetch` API for HTTP requests

**Utilities:**
- `date-fns` - Date formatting and manipulation
- `recharts` - Charting library for stats
- `sonner` - Toast notifications
- `class-variance-authority` - Variant styles for components
- `tailwind-merge` - Merge Tailwind classes intelligently

## ⚡ Performance & Optimization

### Build Optimization

- **Next.js Image Optimization:** Automatic image optimization with `next/image`
- **Code Splitting:** Automatic route-based code splitting with App Router
- **Tree Shaking:** Unused code is eliminated in production builds
- **Minification:** JavaScript, CSS, and HTML minification in production

### Bundle Analysis

```bash
# Analyze bundle size
cd packages/frontend
ANALYZE=true npm run build
```

### Performance Tips

1. **Use dynamic imports for heavy components:**
   ```typescript
   const HeavyChart = dynamic(() => import('./HeavyChart'), { ssr: false })
   ```

2. **Optimize images:** Use WebP format and proper sizing
3. **Lazy load off-screen content:** Use Intersection Observer
4. **Minimize re-renders:** Use React.memo for expensive components
5. **Optimize Web3 calls:** Batch RPC requests when possible

### Caching Strategy

- **Static assets:** Cached indefinitely with hashed filenames
- **API responses:** React Query with 5-minute stale time
- **RPC calls:** Wagmi built-in caching

## 🚢 Deployment

### Production Deployment

The frontend is currently deployed on **Vercel**:
- **Live URL:** [https://shadow-swaps.vercel.app/](https://shadow-swaps.vercel.app/)
- **Auto-deploys:** Pushes to `main` branch trigger automatic deployments
- **Framework Preset:** Next.js

### Deploy Your Own (Vercel)

```bash
# Install Vercel CLI
npm i -g vercel

# From repository root
cd packages/frontend

# Deploy
vercel

# Deploy to production
vercel --prod
```

**Important:** Configure all environment variables in the Vercel dashboard under Project Settings → Environment Variables.

### Alternative: Docker Deployment

```bash
# Build image
docker build -t shadow-swap-frontend -f packages/frontend/Dockerfile .

# Run container
docker run -p 3000:3000 --env-file packages/frontend/.env.local shadow-swap-frontend
```

### Environment Variables for Production

Ensure these critical variables are set in your deployment environment:
- `NEXT_PUBLIC_REOWN_PROJECT_ID` - Wallet connection
- `NEXT_PUBLIC_API_BASE_URL` - Backend API endpoint
- `NEXT_PUBLIC_HMAC_SECRET` - API authentication (use BFF pattern for security)
- All contract addresses for both chains
- RPC URLs (consider using Alchemy or Infura for production)

## 📚 Documentation

- [Next.js Documentation](https://nextjs.org/docs)
- [TailwindCSS](https://tailwindcss.com/docs)
- [shadcn/ui](https://ui.shadcn.com)
- [Reown AppKit](https://docs.reown.com/appkit/overview)
- [Wagmi](https://wagmi.sh)

## 🤝 Contributing

### Development Guidelines

1. **Fork & Clone**
   ```bash
   git clone https://github.com/Mist-Labs/Mantle-Protocol.git
   cd Mantle-Protocol
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b frontend/your-feature-name
   ```

3. **Install Dependencies** (from root)
   ```bash
   make frontend-install
   ```

4. **Make Changes**
   - Follow TypeScript strict mode
   - Use ESLint and Prettier for formatting
   - Keep components modular and reusable
   - Add types for all props and functions

5. **Test Your Changes**
   ```bash
   make frontend-typecheck  # Check TypeScript
   make frontend-lint       # Check linting
   make frontend-build      # Test production build
   ```

6. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat(frontend): add dark mode toggle"
   ```
   Use conventional commits: `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:`

7. **Push & Open PR**
   ```bash
   git push -u origin frontend/your-feature-name
   ```
   Open a Pull Request to the `main` branch

### Code Style Guidelines

- **Components:** Use functional components with TypeScript
- **Hooks:** Create custom hooks in `hooks/` directory
- **Styling:** Use Tailwind CSS classes, avoid inline styles
- **State:** Use React Query for server state, React hooks for local state
- **Naming:**
  - Components: PascalCase (e.g., `BridgeForm.tsx`)
  - Hooks: camelCase with `use` prefix (e.g., `useBridge.ts`)
  - Utils: camelCase (e.g., `formatAddress.ts`)
- **File Structure:** Group related components in feature directories

## 🔧 Troubleshooting

### Common Issues

**Build Errors:**
- Ensure Node.js >= 20.0.0: `node --version`
- Clear cache: `make frontend-clean` then `make frontend-install`
- Check TypeScript errors: `make frontend-typecheck`

**Wallet Connection Issues:**
- Verify `NEXT_PUBLIC_REOWN_PROJECT_ID` is set in `.env.local`
- Check browser console for Web3 errors
- Ensure you're on the correct network (Sepolia testnets)

**Contract Interaction Failures:**
- Verify all contract addresses in `.env.local` match deployed contracts
- Ensure RPC URLs are accessible and not rate-limited
- Check wallet has sufficient testnet funds (use faucets)

**Environment Variable Not Found:**
- All public env vars must start with `NEXT_PUBLIC_`
- Restart dev server after changing `.env.local`
- Verify `.env.local` exists (not just `.env.example`)

**API Connection Issues:**
- Check `NEXT_PUBLIC_API_BASE_URL` is correct
- Verify backend relayer is running and accessible
- Check CORS settings if running locally

## 🌐 Supported Networks

| Network | Chain ID | Status | Block Explorer | Faucet |
|---------|----------|--------|----------------|--------|
| Ethereum Sepolia | 11155111 | ✅ Testnet Live | [sepolia.etherscan.io](https://sepolia.etherscan.io) | [Sepolia Faucet](https://sepoliafaucet.com/) |
| Mantle Sepolia | 5003 | ✅ Testnet Live | [sepolia.mantlescan.xyz](https://sepolia.mantlescan.xyz/) | [Mantle Faucet](https://faucet.sepolia.mantle.xyz/) |
| Ethereum Mainnet | 1 | 🚧 Planned | TBD | - |
| Mantle Mainnet | 5000 | 🚧 Planned | TBD | - |

### Getting Testnet Tokens

1. **Ethereum Sepolia ETH:**
   - [Alchemy Faucet](https://sepoliafaucet.com/)
   - [Infura Faucet](https://www.infura.io/faucet/sepolia)
   - Requires mainnet balance or social verification

2. **Mantle Sepolia MNT:**
   - [Official Mantle Faucet](https://faucet.sepolia.mantle.xyz/)
   - Bridge Sepolia ETH to Mantle Sepolia

3. **Testnet USDC/USDT:**
   - Use token faucets or testnet DEXs
   - Check contract addresses in `.env.example`

## 🏛️ Architecture

This frontend interfaces with:
- **Smart Contracts:** Deployed on both Ethereum Sepolia and Mantle Sepolia
- **Relayer Backend:** Rust-based backend (packages/shadow-swap) for proof generation and root syncing
- **Solver Network:** ERC-7683 compatible intent fulfillment system
- **Indexer:** Event monitoring via Goldsky webhooks (packages/Indexers)

## 🔗 Links

- **Live App:** [https://shadow-swaps.vercel.app/](https://shadow-swaps.vercel.app/)
- **GitHub Repository:** [https://github.com/Mist-Labs/Mantle-Protocol](https://github.com/Mist-Labs/Mantle-Protocol)
- **Monorepo Root:** [See main README](../../README.md) for full project documentation
