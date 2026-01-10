# 🌉 **Shadow Swap - Privacy-Preserving Intent Bridge**
### **Zero-Knowledge Bridge Protocol for Cross-Chain Asset Transfers**

| **Status** | **Phase 1 (Privacy Bridge MVP)** | **Network** | **Ethereum ↔ Mantle L2** |
|-----------|-----------------------------------|-------------|---------------------------|
| **Complexity** | 8/10 (Commitment-based Privacy) | **Repo Type** | Monorepo (Yarn Workspaces) |
| **Solidity Tooling** | Foundry | **Backend** | Rust (Actix-Web) |

---

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
| **Smart Contracts** | `@mantle/contracts` | Intent pools, settlement, merkle verification, Poseidon hasher | Solidity (Foundry) |
| **Frontend dApp** | `@mantle/frontend` | User interface for private bridging | Next.js, TypeScript, Ethers.js |
| **Relayer Backend** | `@mantle/shadow-swap` | Off-chain proof generation, root syncing, settlement coordination | Rust (Actix-Web, PostgreSQL, Diesel) |
| **Event Indexer** | `@mantle/Indexers` | Blockchain event monitoring via Goldsky webhooks | Node.js, TypeScript, BullMQ |
| **Solver Bot** | `@mantle/solver` | Intent fulfillment and liquidity provision | Node.js, TypeScript |

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

This is a **monorepo** managed with **Yarn Workspaces**.  
Run all commands from the **repo root**.

---

### **3.1. Getting Started**

#### Clone the Repository
```bash
git clone https://github.com/Mist-Labs/Mantle-Protocol.git
cd Mantle-Protocol
```

#### Install Dependencies
```bash
yarn install
```

---

### **3.2. Package Commands**

| Action | Package | Command |
|--------|---------|---------|
| **Compile Contracts** | `@mantle/contracts` | `yarn workspace @mantle/contracts build` |
| **Run Contract Tests** | `@mantle/contracts` | `yarn workspace @mantle/contracts test` |
| **Deploy Contracts** | `@mantle/contracts` | `yarn workspace @mantle/contracts deploy` |
| **Run Frontend** | `@mantle/shadow-swap` | `yarn workspace @mantle/shadow-swap dev` |
| **Run Indexer** | `@mantle/indexer` | `yarn workspace @mantle/indexer start` |
| **Run Solver** | `@mantle/solver` | `yarn workspace @mantle/solver start` |
| **Run Relayer** | `@mantle/relayer` | `cd packages/relayer && cargo run --release` |

---

### **3.3. Environment Setup**

#### **Contracts** (`.env` in `packages/contracts/`)
```bash
ETHEREUM_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
PRIVATE_KEY=0x...
ETHERSCAN_API_KEY=...
```

#### **Relayer** (`.env` in `packages/relayer/`)
```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/shadow_swap
ETHEREUM_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
MANTLE_RPC_URL=https://rpc.sepolia.mantle.xyz
RELAYER_PRIVATE_KEY=0x...
HMAC_SECRET=...
```

#### **Indexer** (`.env` in `packages/indexer/`)
```bash
GOLDSKY_WEBHOOK_SECRET=...
RELAYER_BASE_URL=http://localhost:8080
HMAC_SECRET=...
```

---

### **3.4. Running the Full Stack**
```bash
# Terminal 1: Start PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres:14

# Terminal 2: Start Relayer
cd packages/relayer && cargo run --release

# Terminal 3: Start Indexer
yarn workspace @mantle/indexer start

# Terminal 4: Start Solver
yarn workspace @mantle/solver start

# Terminal 5: Start Frontend
yarn workspace @mantle/shadow-swap dev
```

---

## 📦 **3.5. Dependency Management**

⚠️ **Never run `yarn add` inside individual package folders.**  
Always add dependencies using **workspace commands** from the repo root.

| Action | Command |
|--------|---------|
| Add Dependency | `yarn workspace @mantle/shadow-swap add axios` |
| Add Dev Dependency | `yarn workspace @mantle/contracts add -D solhint` |
| Link Local Packages | Automatic after `yarn install` |

---

## 🧪 **4. Testing**

### **Smart Contracts**
```bash
# Unit tests
yarn workspace @mantle/contracts test

# Integration tests
yarn workspace @mantle/contracts test:integration

# Gas reports
yarn workspace @mantle/contracts test -- --gas-report

# Coverage
yarn workspace @mantle/contracts coverage
```

### **Relayer**
```bash
cd packages/relayer
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
- **Security Contact**: security@mistlabs.xyz

### Known Limitations (Phase 1)
- Centralized relayer (decentralization planned for Phase 2)
- No ZK circuits yet (commitments use Poseidon but no ZK proofs)
- Testnet only (mainnet after audit)

---

## 📜 **License**

MIT License - See [LICENSE](./LICENSE) for details

---

## 🌐 **Links**

- **Website**: https://shadowswap.xyz
- **Docs**: https://docs.shadowswap.xyz
- **Discord**: https://discord.gg/shadowswap
- **Twitter**: https://twitter.com/shadowswap

---

Built with ❤️ by **Mist Labs**