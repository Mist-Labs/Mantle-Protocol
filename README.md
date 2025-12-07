# 🌉 **Mantle EthBridge (Hyper-Light Protocol)**
### **Privacy-Enhanced, Capital-Efficient Bridging Protocol for the Mantle Network**

| **Status** | **Phase 1 (Optimistic MVP)** | **Network** | **Mantle L2 / Ethereum L1** |
|-----------|-------------------------------|-------------|------------------------------|
| **Complexity** | 7/10 (Optimistic Verification) | **Repo Type** | Monorepo (Yarn Workspaces) |
| **Solidity Tooling** | Foundry (Solidity) | **Front-end** | Next.js / TypeScript |

---

## 💡 **1. Project Overview**

The **Mantle EthBridge** is a modular, next-generation bridging solution designed to facilitate **secure**, **low-latency**, and **capital-efficient** asset transfers between **Ethereum L1** and **Mantle L2**.

The project follows a **strategic, phased rollout**:

---

### 🔹 **Phase 1 (MVP): Optimistic Verification**
Launch with **ERC-7683 compatibility** and an **Optimistic Verification Layer** to provide:
- Fast settlement  
- Solver network access  
- Low-latency bridging  
- Immediate market utility  

### 🔹 **Phase 2 (Final State): Zero-Knowledge Verification**
Upgrade to a **fully trustless**, ZK-enabled Hyperbridge with:
- On-chain ZK state transition proofs  
- Censorship-resistant settlement  
- Permissionless provers + slashing  

---

## 🧩 **Core Components in This Monorepo**

| Component | Directory | Description | Technology |
|----------|-----------|-------------|------------|
| **Smart Contracts** | `packages/contracts` | ERC-7683 implementation, Optimistic Verifier, PrivacySuite | Solidity, Foundry |
| **Frontend dApp** | `packages/shadow-swap` | User interface for bridging, swapping, and privacy actions | Next.js, React, TS |
| **Backend Services** | `packages/shadow-swap/backend` | Off-chain relayer, API services, settlement logic | Rust (Actix-Web) |
| **Indexers** | `packages/shadow-swap/indexers` | Event log monitoring and protocol data sync | Node.js, TypeScript |

---

## 🗺️ **2. Implementation Strategy & Roadmap**

### **Phase 1 — MVP (4–6 Weeks)**
**Goal:** Functional optimistic bridge  
| Area | Details |
|------|---------|
| Market Access | Implement ERC-7683 for solver liquidity access (Across, Connext, etc.) |
| Verification | Simplified optimistic model + short `CHALLENGE_PERIOD` |
| Relayer | Centralized relayer submitting batched proofs |
| Outcome | Fast, capital-efficient bridging on Mantle |

---

### **Phase 2 — Trustless (12+ Weeks)**
**Goal:** Transform into a fully trustless Hyperbridge  
| Area | Details |
|------|---------|
| Upgrade | Replace optimistic verifier with a ZK verifier contract |
| Security | Implement ZK circuits for cross-chain state validation |
| Decentralization | Permissionless provers + staking + slashing |
| Outcome | High-security, censorship-resistant protocol |

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
---

## 🚀 Install Dependencies

```bash
yarn install
```

## 🔧 3.2. Running Workspace Commands

Use the following format:
`yarn workspace <package-name> <script>`

### **Examples**

| Action               | Package               | Command                                                     |
|----------------------|------------------------|-------------------------------------------------------------|
| Compile Contracts    | `@mantle/contracts`    | `yarn workspace @mantle/contracts compile`                 |
| Run Contract Tests   | `@mantle/contracts`    | `yarn workspace @mantle/contracts test -- -vv`             |
| Run Frontend         | `@mantle/shadow-swap`  | `yarn workspace @mantle/shadow-swap dev`                   |
| Run Rust Backend     | backend                | `cd packages/shadow-swap/backend && cargo run`             |


📦 **3.3. Dependency Management**

⚠️ **Never run `yarn add` inside individual package folders.**  
Always add dependencies using **workspace commands** from the repo root.

### Commands

| **Action**             | **Command**                                               |
|------------------------|-----------------------------------------------------------|
| Add Dependency         | `yarn workspace @mantle/shadow-swap add axios`            |
| Add Dev Dependency     | `yarn workspace @mantle/contracts add -D solhint`         |
| Link Local Packages    | Automatically handled after `yarn install`                |


🤝 **4. Contribution Guide**

This monorepo uses a single Git repository located at the root.

### Create a New Branch
```bash
git checkout -b feat/describe-your-change
```

### Commit Your Work (from root)
```bash
git add .
git commit -m "feat: Implement challenge function in OptimisticVerifier contract"
```

### Push & Open Pull Request
```bash
git push -u origin feat/describe-your-change
```

Then open a Pull Request to the **main** branch on GitHub.
