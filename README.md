# 🌉 **Shadow Swap - Privacy-Preserving Intent Bridge**
### **Zero-Knowledge Bridge Protocol for Cross-Chain Asset Transfers**

| **Status** | **Phase 1 (Privacy Bridge MVP)** | **Network** | **Ethereum ↔ Mantle L2** |
|-----------|-----------------------------------|-------------|---------------------------|
| **Complexity** | 8/10 (Commitment-based Privacy) | **Repo Type** | Monorepo |
| **Solidity Tooling** | Foundry | **Backend** | Rust (Actix-Web) |

---

#### Launch project - [Shadow-Swap](https://shadow-swaps.vercel.app/)

## 💡 **1. Project Overview**

**Shadow Swap** is a privacy-preserving, intent-based bridging protocol that enables **anonymous** and **trustless** asset transfers between **Ethereum L1** and **Mantle L2**.

Unlike traditional bridges, Shadow Swap **decouples the sender from the receiver** using cryptographic commitments, allowing users to:
- Bridge assets **privately** (no on-chain link between depositor and withdrawer)
- Achieve **immediate settlement** (no challenge periods or delays)
- Maintain **censorship resistance** through permissionless solver networks

The project follows a **strategic, phased rollout**:

---

### 🔹 **Phase 1 (Current): Privacy Bridge MVP**
Fully functional privacy bridge with:
- **Poseidon hash-based commitments** for privacy
- **Merkle proof verification** for cross-chain state validation
- **Intent-based architecture** compatible with ERC-7683
- **Solver network** for decentralized fulfillment
- **Immediate settlement** (no waiting periods)

### 🔹 **Phase 2 (Planned): Zero-Knowledge Upgrade**
Enhance with full ZK infrastructure:
- **zk-SNARK circuits** for transaction privacy
- **Recursive proof aggregation** for scalability
- **Nullifier-based double-spend prevention** (on-chain ZK verification)
- **Multi-chain expansion** beyond Ethereum ↔ Mantle

---

## 🏗️ **Architecture Overview**

### **Intent Lifecycle**
```
┌─────────────────────────────────────────────────────────────────┐
│  1. USER CREATES INTENT (Source Chain)                          │
│     • Generates secret + nullifier (off-chain)                   │
│     • Computes commitment = Poseidon(secret, nullifier, ...)     │
│     • Deposits tokens to PrivateIntentPool contract              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  2. INDEXER MONITORS EVENTS                                      │
│     • Goldsky webhook receives IntentCreated event               │
│     • Relayer builds merkle tree of commitments                  │
│     • Syncs commitment root to destination chain                 │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  3. RELAYER REGISTERS INTENT (Destination Chain)                 │
│     • Generates merkle proof for commitment                      │
│     • Calls PrivateSettlement.registerIntent() with proof        │
│     • Intent becomes fillable on destination chain               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  4. SOLVER FILLS INTENT (Destination Chain)                      │
│     • Monitors IntentRegistered events                           │
│     • Provides liquidity by calling fillIntent()                 │
│     • Tokens sent to recipient address                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  5. SETTLEMENT (Source Chain)                                    │
│     • Relayer syncs fill root back to source chain               │
│     • Solver claims escrowed funds via settleIntent()            │
│     • User privately claims with secret/nullifier                │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🧩 **Core Components**

| Component | Package | Description | Technology |
|-----------|---------|-------------|------------|
| **Smart Contracts** | `@Mantle/contracts` | Intent pools, settlement, merkle verification, Poseidon hasher | Solidity (Foundry) |
| **Frontend dApp** | `@Mantle/frontend` | User interface for private bridging | Next.js, TypeScript, Ethers.js |
| **Relayer Backend** | `@Mantle/shadow-swap` | Off-chain proof generation, root syncing, settlement coordination | Rust (Actix-Web, PostgreSQL, Diesel) |
| **Event Indexer** | `@Mantle/Indexers` | Blockchain event monitoring via Goldsky webhooks | Node.js, TypeScript, BullMQ |
| **Solver Bot** | `@Mantle/solver` | Intent fulfillment and liquidity provision | Rust (Actix-Web) |

---

## 🔐 **Privacy Model**

### **Commitment Scheme**
```solidity
commitment = Poseidon(secret, nullifier, amount, destChain)
```

- **Secret**: Random 256-bit value (known only to user)
- **Nullifier**: Prevents double-spending when claiming
- **Commitment**: Public identifier stored on-chain
- **No on-chain link** between depositor and withdrawer

### **Merkle Proof System**
- All commitments aggregated into merkle tree (minimum size 2)
- Root synced cross-chain for verification
- Proofs generated off-chain, verified on-chain
- Supports batching for gas efficiency

---

## 🗺️ **2. Roadmap**

### **Phase 1 — Privacy Bridge MVP** ✅ **COMPLETE**
| Milestone | Status |
|-----------|--------|
| Poseidon commitment contracts | ✅ Deployed |
| Merkle proof verification | ✅ Working |
| Intent-based architecture | ✅ Live |
| Relayer infrastructure | ✅ Operational |
| Solver network integration | ✅ ERC-7683 compatible |
| Testnet deployment (Sepolia) | ✅ Live |

---

### **Phase 2 — Zero-Knowledge Upgrade** 🚧 **PLANNED**
| Milestone | Timeline |
|-----------|----------|
| zk-SNARK circuit design | Q2 2025 |
| Nullifier ZK verification | Q2 2025 |
| Recursive proof aggregation | Q3 2025 |
| Multi-chain expansion | Q3 2025 |
| Mainnet launch | Q4 2025 |

---

## 👩‍💻 **3. Developer Guide**

This is a **monorepo** with multiple packages.  
Each package can be run independently from its directory.

---

### **3.1. Getting Started**

#### Clone the Repository
```bash
git clone https://github.com/Mist-Labs/Mantle-Protocol.git
cd Mantle-Protocol
```

#### Install Dependencies

Each package manages its own dependencies:

```bash
# Smart Contracts
cd packages/contracts && forge install

# Frontend
cd packages/frontend && pnpm install

# Indexer
cd packages/Indexers && pnpm install

# Relayer (Rust dependencies via Cargo.toml)
cd packages/shadow-swap && cargo build

# Solver (Rust dependencies via Cargo.toml)
cd packages/solver && cargo build
```

---

### **3.2. Package Commands**

| Action | Package | Command |
|--------|---------|---------|
| **Compile Contracts** | `@Mantle/contracts` | `cd packages/contracts && forge build` |
| **Run Contract Tests** | `@Mantle/contracts` | `cd packages/contracts && ./runtests.sh` |
| **Deploy Contracts** | `@Mantle/contracts` | `cd packages/contracts && forge script script/Deployer.s.sol` |
| **Run Frontend** | `@Mantle/frontend` | `cd packages/frontend && pnpm run dev` |
| **Run Indexer** | `@Mantle/Indexers` | `cd packages/Indexers && pnpm run dev` |
| **Run Solver** | `@Mantle/solver` | `cd packages/solver && cargo run --release` |
| **Run Relayer** | `@Mantle/shadow-swap` | `cd packages/shadow-swap && cargo run --release` |

---

### **3.3. Environment Setup**

#### **Contracts** (`.env` in `packages/contracts/`)
```bash
check .env.example 
```

#### **Relayer** (`.env` in `packages/relayer/`)
```bash
check .env.example 
```

#### **frontend** (`.env` in `packages/frontend/`)
```bash
check .env.example 
```

---

### **3.4. Running the Full Stack**
```bash
# Terminal 1: Start PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres:14

# Terminal 2: Start Relayer
cd packages/shadow-swap && cargo run --release

# Terminal 3: Start Indexer
cd packages/Indexers && pnpm run dev

# Terminal 4: Start Solver
cd packages/solver && cargo run --release

# Terminal 5: Start Frontend
cd packages/frontend && pnpm run dev
```

---

## 📦 **3.5. Dependency Management**

### **Smart Contracts (Foundry)**
```bash
cd packages/contracts
forge install <dependency>  # Install Foundry library
forge update                # Update dependencies
```

### **Frontend & Indexer (pnpm)**
```bash
cd packages/frontend  # or packages/Indexers
pnpm add <package>         # Add dependency
pnpm add -D <package>      # Add dev dependency
pnpm install               # Install dependencies
```

### **Rust Services (Cargo)**
```bash
cd packages/shadow-swap  # or packages/solver
cargo add <crate>          # Add dependency
cargo update               # Update dependencies
```

---

## 🧪 **4. Testing**

### **Smart Contracts**
```bash
# Unit tests
cd packages/contracts && ./runtests.sh

# Integration tests
cd packages/contracts && forge test --match-path test/integration/*

# Gas reports
cd packages/contracts && forge test --gas-report

# Coverage
cd packages/contracts && forge coverage
```

### **Relayer**
```bash
cd packages/shadow-swap
cargo test
cargo test -- --nocapture  # With logs
```

---

## 🚀 **5. Deployment**

### **Testnet (Sepolia)**
```bash
# Deploy Poseidon hasher
forge script script/Deployer.s.sol:DeployPoseidonHasher \
  --rpc-url $ETHEREUM_RPC_URL --broadcast --verify

# Deploy Ethereum contracts
forge script script/Deployer.s.sol:DeployEthereumContracts \
  --rpc-url $ETHEREUM_RPC_URL --broadcast --verify

# Deploy Mantle contracts
forge script script/Deployer.s.sol:DeployMantleContracts \
  --rpc-url $MANTLE_RPC_URL --broadcast --verify

# Configure tokens
forge script script/Deployer.s.sol:ConfigureTokens \
  --rpc-url $ETHEREUM_RPC_URL --broadcast
```

---

## 🏛️ **6. Smart Contract Architecture**

### **Core Contracts**

| Contract | Chain | Purpose |
|----------|-------|---------|
| `PrivateIntentPool` | Source | Intent creation, escrow, settlement |
| `PrivateSettlement` | Destination | Intent registration, filling, claiming |
| `PoseidonHasher` | Both | Commitment generation |

### **Key Functions**

**PrivateIntentPool (Source Chain)**
```solidity
createIntent(commitment, token, amount, destChain) // Escrow tokens
settleIntent(intentId, merkleProof) // Solver claims funds
claimWithdrawal(secret, nullifier, signature) // Private claim
```

**PrivateSettlement (Destination Chain)**
```solidity
registerIntent(commitment, token, amount, merkleProof) // Register via relayer
fillIntent(intentId) // Solver provides liquidity
```

---

## 🤝 **7. Contributing**

### Create a Branch
```bash
git checkout -b feat/your-feature-name
```

### Commit Changes
```bash
git add .
git commit -m "feat: add zkSNARK nullifier verification"
```

### Push & Open PR
```bash
git push -u origin feat/your-feature-name
```

Then open a Pull Request to **main** on GitHub.

---

## 📊 **8. Current Network Support**

| Network | Chain ID | Status | Contracts Deployed |
|---------|----------|--------|--------------------|
| Ethereum Sepolia | 11155111 | ✅ Live | PrivateIntentPool, PrivateSettlement, PoseidonHasher |
| Mantle Sepolia | 5003 | ✅ Live | PrivateIntentPool, PrivateSettlement, PoseidonHasher |
| Ethereum Mainnet | 1 | 🚧 Planned | TBD |
| Mantle Mainnet | 5000 | 🚧 Planned | TBD |

---

## 🔒 **9. Security**

- **Audits**: Pending (Phase 2)
- **Bug Bounty**: TBD
- **Security Contact**: ebounce500@gmail.com

### Known Limitations (Phase 1)
- Centralized relayer (decentralization planned for Phase 2)
- No ZK circuits yet (commitments use Poseidon but no ZK proofs)
- Testnet only (mainnet after audit)

---

## 📜 **License**

MIT License - See [LICENSE](./LICENSE) for details

---

## 🌐 **Links**

- **Website**: coming soon
- **Docs**: coming soon
- **Discord**: coming soon
- **Twitter**: coming soon

---

Built with ❤️ by **Mist Labs**