# VeilWallet

A self-custodial smart contract wallet with privacy features, built on Mantle using ERC-4337 account abstraction.

## What is This?

VeilWallet is a smart contract wallet system that lets users:
- Create smart contract wallets (not regular EOA wallets)
- Use session keys for limited, time-bound access
- Recover accounts via guardians
- Send private token transfers (amounts/recipients hidden on-chain)

---

## 🌐 Network Configuration

**Network**: Mantle Sepolia Testnet  
**Chain ID**: 5003  
**RPC URL**: `https://rpc.sepolia.mantle.xyz`  
**Explorer**: `https://explorer.sepolia.mantle.xyz`  
**Native Currency**: MNT (Mantle)

---

## 📍 Deployed Contract Addresses

| Contract | Address | Explorer |
|----------|---------|----------|
| **AccountFactory** | `0x55633aFf235600374Ef58D2A5e507Aa39C9e0D37` | [View](https://explorer.sepolia.mantle.xyz/address/0x55633aff235600374ef58d2a5e507aa39c9e0d37) |
| **VeilToken** | `0xc9620e577D0C43B5D09AE8EA406eced818402739` | [View](https://explorer.sepolia.mantle.xyz/address/0xc9620e577d0c43b5d09ae8ea406eced818402739) |
| **Verifier** | `0x5ba2d923f8b1E392997D87060E207E1BAAeA3E13` | [View](https://explorer.sepolia.mantle.xyz/address/0x5ba2d923f8b1e392997d87060e207e1baaea3e13) |
| **PoseidonHasher** | `0x7ff31538A93950264e26723C959a9D196bfB9779` | [View](https://explorer.sepolia.mantle.xyz/address/0x7ff31538a93950264e26723c959a9d196bfb9779) |
| **EntryPoint** | `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789` | [View](https://explorer.sepolia.mantle.xyz/address/0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789) |

**Note**: EntryPoint is the canonical ERC-4337 address, same on all chains.

---

## 📦 Contracts

### 1. AccountFactory.sol

**Address**: `0x55633aFf235600374Ef58D2A5e507Aa39C9e0D37`

Creates new smart contract wallets for users using CREATE2 for predictable addresses.

#### Functions

```solidity
// Get predicted address without deploying
function getAddress(address owner, uint256 salt) public view returns (address)

// Deploy a new wallet (or return existing if already deployed)
function createAccount(address owner, uint256 salt) external returns (SmartAccount)
```

#### Events

```solidity
event AccountCreated(address indexed account, address indexed owner, uint256 salt)
```

#### Usage Example

```typescript
import { ethers } from 'ethers';

const factoryAddress = '0x55633aFf235600374Ef58D2A5e507Aa39C9e0D37';
const factoryABI = [
  'function getAddress(address owner, uint256 salt) public view returns (address)',
  'function createAccount(address owner, uint256 salt) external returns (address)',
  'event AccountCreated(address indexed account, address indexed owner, uint256 salt)'
];

const provider = new ethers.JsonRpcProvider('https://rpc.sepolia.mantle.xyz');
const factory = new ethers.Contract(factoryAddress, factoryABI, provider);

// Get predicted address
const owner = '0x...'; // User's EOA address
const salt = ethers.keccak256(ethers.toUtf8Bytes('user@example.com'));
const predictedAddress = await factory.getAddress(owner, salt);

// Deploy account (if not already deployed)
const signer = new ethers.Wallet(privateKey, provider);
const factoryWithSigner = factory.connect(signer);
const tx = await factoryWithSigner.createAccount(owner, salt);
await tx.wait();
```

---

### 2. SmartAccount.sol

**Address**: Deployed per user (use AccountFactory to get address)

The actual wallet contract. Users interact with this for all operations.

#### Functions

**Account Management**:
```solidity
function owner() public view returns (address)
function signer() public view returns (address)
function entryPoint() public view returns (IEntryPoint)
function changeOwner(address newOwner) external
```

**Execution**:
```solidity
function execute(address target, uint256 value, bytes calldata data) external payable
function executeBatch(address[] calldata targets, uint256[] calldata values, bytes[] calldata datas) external payable
```

**Session Keys**:
```solidity
function addSessionKey(address sessionKey, uint256 validUntil, uint256 spendingLimit) external
function revokeSessionKey(address sessionKey) external
function sessionKeys(address) public view returns (uint256 validUntil, uint256 spendingLimit, uint256 spentAmount)
```

**Guardian Recovery**:
```solidity
function setGuardian(address _guardian) external
function initiateRecovery(address newOwner) external
function executeRecovery() external
function guardian() public view returns (address)
function pendingRecovery() public view returns (address newOwner, uint256 timestamp, bool executed)
```

**ERC-4337**:
```solidity
function validateUserOp(UserOperation calldata userOp, bytes32 userOpHash, uint256 missingAccountFunds) external returns (uint256 validationData)
```

#### Events

```solidity
event SessionKeyAdded(address indexed sessionKey, uint256 validUntil, uint256 spendingLimit)
event SessionKeyRevoked(address indexed sessionKey)
event RecoveryInitiated(address indexed newOwner, uint256 timestamp)
event RecoveryExecuted(address indexed oldOwner, address indexed newOwner)
event OwnerChanged(address indexed oldOwner, address indexed newOwner)
```

#### Usage Examples

**Execute Transaction**:
```typescript
const accountAddress = '0x...'; // SmartAccount address
const accountABI = [
  'function execute(address target, uint256 value, bytes calldata data) external payable'
];

const account = new ethers.Contract(accountAddress, accountABI, signer);

// Transfer tokens
const tokenAddress = '0x...';
const data = tokenInterface.encodeFunctionData('transfer', [recipient, amount]);
const tx = await account.execute(tokenAddress, 0, data);
await tx.wait();
```

**Add Session Key**:
```typescript
const accountABI = [
  'function addSessionKey(address sessionKey, uint256 validUntil, uint256 spendingLimit) external'
];

const account = new ethers.Contract(accountAddress, accountABI, signer);

const sessionKey = '0x...'; // Session key address
const validUntil = Math.floor(Date.now() / 1000) + (7 * 24 * 60 * 60); // 7 days
const spendingLimit = ethers.parseEther('1.0'); // 1 MNT limit

const tx = await account.addSessionKey(sessionKey, validUntil, spendingLimit);
await tx.wait();
```

**Set Guardian**:
```typescript
const accountABI = [
  'function setGuardian(address _guardian) external'
];

const account = new ethers.Contract(accountAddress, accountABI, signer);
const tx = await account.setGuardian(guardianAddress);
await tx.wait();
```

---

### 3. VeilToken.sol

**Address**: `0xc9620e577D0C43B5D09AE8EA406eced818402739`

Privacy-preserving ERC20 token with standard and private transfer capabilities.

#### Functions

**Standard ERC20**:
```solidity
function transfer(address to, uint256 amount) public returns (bool)
function transferFrom(address from, address to, uint256 amount) public returns (bool)
function approve(address spender, uint256 amount) public returns (bool)
function balanceOf(address account) public view returns (uint256)
function totalSupply() public view returns (uint256)
```

**Private Transfers**:
```solidity
function privateTransfer(bytes32 commitment, bytes32 nullifier, uint256 amount, bytes calldata proof) external
function claimFromCommitment(bytes32 commitment, uint256 amount, bytes calldata proof) external
function createCommitment(bytes32[4] calldata inputs) external view returns (bytes32)
function isNullifierUsed(bytes32 nullifier) external view returns (bool)
function isCommitmentValid(bytes32 commitment) external view returns (bool)
```

**Minting**:
```solidity
function mint(address to, uint256 amount) external
```

#### Events

```solidity
event Transfer(address indexed from, address indexed to, uint256 value) // Standard ERC20
event Approval(address indexed owner, address indexed spender, uint256 value) // Standard ERC20
event PrivateTransfer(bytes32 indexed commitment, bytes32 indexed nullifier, address indexed recipient)
event CommitmentClaimed(bytes32 indexed commitment, address indexed recipient, uint256 amount)
event EncryptedBalanceUpdated(address indexed account, bytes encryptedData)
```

#### Usage Examples

**Standard Transfer**:
```typescript
const tokenAddress = '0xc9620e577D0C43B5D09AE8EA406eced818402739';
const tokenABI = [
  'function transfer(address to, uint256 amount) public returns (bool)',
  'function balanceOf(address account) public view returns (uint256)'
];

const token = new ethers.Contract(tokenAddress, tokenABI, signer);

// Transfer
const tx = await token.transfer(recipient, ethers.parseEther('100'));
await tx.wait();

// Check balance
const balance = await token.balanceOf(userAddress);
```

**Private Transfer**:
```typescript
const verifierAddress = '0x5ba2d923f8b1E392997D87060E207E1BAAeA3E13';
const verifierABI = [
  'function verifyCommitment(bytes32[4] calldata inputs) external view returns (bytes32)'
];

const verifier = new ethers.Contract(verifierAddress, verifierABI, provider);

// Create commitment inputs
const amount = ethers.parseEther('50');
const blinding = ethers.randomBytes(32);
const recipient = ethers.zeroPadValue(recipientAddress, 32);
const nonce = ethers.randomBytes(32);

const inputs: [string, string, string, string] = [
  ethers.zeroPadValue(ethers.toBeHex(amount), 32),
  ethers.hexlify(blinding),
  recipient,
  ethers.hexlify(nonce)
];

// Get commitment
const commitment = await verifier.verifyCommitment(inputs);

// Create nullifier
const nullifier = ethers.keccak256(
  ethers.concat([commitment, ethers.toUtf8Bytes('nullifier')])
);

// Create proof (encode inputs)
const proof = ethers.AbiCoder.defaultAbiCoder().encode(
  ['bytes32[4]'],
  [inputs]
);

// Execute private transfer
const tokenABI = [
  'function privateTransfer(bytes32 commitment, bytes32 nullifier, uint256 amount, bytes calldata proof) external'
];

const token = new ethers.Contract(tokenAddress, tokenABI, signer);
const tx = await token.privateTransfer(commitment, nullifier, amount, proof);
await tx.wait();
```

**Claim Private Transfer**:
```typescript
const tokenABI = [
  'function claimFromCommitment(bytes32 commitment, uint256 amount, bytes calldata proof) external'
];

const token = new ethers.Contract(tokenAddress, tokenABI, signer);

// Recipient claims (proof is placeholder for MVP)
const proof = ethers.toUtf8Bytes('proof'); // In production, this would be a ZK proof
const tx = await token.claimFromCommitment(commitment, amount, proof);
await tx.wait();
```

---

### 4. Verifier.sol

**Address**: `0x5ba2d923f8b1E392997D87060E207E1BAAeA3E13`

Verifies Poseidon hash commitments for private transfers.

#### Functions

```solidity
function verifyCommitment(bytes32[4] calldata inputs) external view returns (bytes32)
function verifyCommitment2(bytes32[2] calldata inputs) external view returns (bytes32)
function verifyCommitment3(bytes32[3] calldata inputs) external view returns (bytes32)
function verifyCommitmentMatch(bytes32[4] calldata inputs, bytes32 expectedCommitment) external view returns (bool)
```

#### Usage

```typescript
const verifierAddress = '0x5ba2d923f8b1E392997D87060E207E1BAAeA3E13';
const verifierABI = [
  'function verifyCommitment(bytes32[4] calldata inputs) external view returns (bytes32)'
];

const verifier = new ethers.Contract(verifierAddress, verifierABI, provider);

const inputs: [string, string, string, string] = [
  amountBytes,
  blindingBytes,
  recipientBytes,
  nonceBytes
];

const commitment = await verifier.verifyCommitment(inputs);
```

---

## 🔧 Frontend Integration

### TypeScript/JavaScript Setup

```typescript
import { ethers } from 'ethers';

// Network configuration
const CONFIG = {
  chainId: 5003,
  rpcUrl: 'https://rpc.sepolia.mantle.xyz',
  explorer: 'https://explorer.sepolia.mantle.xyz',
  contracts: {
    entryPoint: '0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789',
    accountFactory: '0x55633aFf235600374Ef58D2A5e507Aa39C9e0D37',
    veilToken: '0xc9620e577D0C43B5D09AE8EA406eced818402739',
    verifier: '0x5ba2d923f8b1E392997D87060E207E1BAAeA3E13',
    poseidonHasher: '0x7ff31538A93950264e26723C959a9D196bfB9779'
  }
};

// Initialize provider
const provider = new ethers.JsonRpcProvider(CONFIG.rpcUrl);

// Initialize contracts
const factory = new ethers.Contract(
  CONFIG.contracts.accountFactory,
  FACTORY_ABI,
  provider
);

const token = new ethers.Contract(
  CONFIG.contracts.veilToken,
  TOKEN_ABI,
  provider
);
```

### Complete ABI Arrays

**AccountFactory ABI**:
```typescript
const FACTORY_ABI = [
  'function getAddress(address owner, uint256 salt) public view returns (address)',
  'function createAccount(address owner, uint256 salt) external returns (address)',
  'function entryPoint() public view returns (address)',
  'event AccountCreated(address indexed account, address indexed owner, uint256 salt)'
];
```

**SmartAccount ABI**:
```typescript
const ACCOUNT_ABI = [
  'function owner() public view returns (address)',
  'function signer() public view returns (address)',
  'function entryPoint() public view returns (address)',
  'function execute(address target, uint256 value, bytes calldata data) external payable',
  'function executeBatch(address[] calldata targets, uint256[] calldata values, bytes[] calldata datas) external payable',
  'function addSessionKey(address sessionKey, uint256 validUntil, uint256 spendingLimit) external',
  'function revokeSessionKey(address sessionKey) external',
  'function sessionKeys(address) public view returns (uint256 validUntil, uint256 spendingLimit, uint256 spentAmount)',
  'function setGuardian(address _guardian) external',
  'function initiateRecovery(address newOwner) external',
  'function executeRecovery() external',
  'function guardian() public view returns (address)',
  'function pendingRecovery() public view returns (address newOwner, uint256 timestamp, bool executed)',
  'function changeOwner(address newOwner) external',
  'event SessionKeyAdded(address indexed sessionKey, uint256 validUntil, uint256 spendingLimit)',
  'event SessionKeyRevoked(address indexed sessionKey)',
  'event RecoveryInitiated(address indexed newOwner, uint256 timestamp)',
  'event RecoveryExecuted(address indexed oldOwner, address indexed newOwner)',
  'event OwnerChanged(address indexed oldOwner, address indexed newOwner)'
];
```

**VeilToken ABI**:
```typescript
const TOKEN_ABI = [
  // Standard ERC20
  'function transfer(address to, uint256 amount) public returns (bool)',
  'function transferFrom(address from, address to, uint256 amount) public returns (bool)',
  'function approve(address spender, uint256 amount) public returns (bool)',
  'function balanceOf(address account) public view returns (uint256)',
  'function totalSupply() public view returns (uint256)',
  'function allowance(address owner, address spender) public view returns (uint256)',
  // Private transfers
  'function privateTransfer(bytes32 commitment, bytes32 nullifier, uint256 amount, bytes calldata proof) external',
  'function claimFromCommitment(bytes32 commitment, uint256 amount, bytes calldata proof) external',
  'function createCommitment(bytes32[4] calldata inputs) external view returns (bytes32)',
  'function isNullifierUsed(bytes32 nullifier) external view returns (bool)',
  'function isCommitmentValid(bytes32 commitment) external view returns (bool)',
  'function mint(address to, uint256 amount) external',
  // Events
  'event Transfer(address indexed from, address indexed to, uint256 value)',
  'event Approval(address indexed owner, address indexed spender, uint256 value)',
  'event PrivateTransfer(bytes32 indexed commitment, bytes32 indexed nullifier, address indexed recipient)',
  'event CommitmentClaimed(bytes32 indexed commitment, address indexed recipient, uint256 amount)',
  'event EncryptedBalanceUpdated(address indexed account, bytes encryptedData)'
];
```

**Verifier ABI**:
```typescript
const VERIFIER_ABI = [
  'function verifyCommitment(bytes32[4] calldata inputs) external view returns (bytes32)',
  'function verifyCommitment2(bytes32[2] calldata inputs) external view returns (bytes32)',
  'function verifyCommitment3(bytes32[3] calldata inputs) external view returns (bytes32)',
  'function verifyCommitmentMatch(bytes32[4] calldata inputs, bytes32 expectedCommitment) external view returns (bool)',
  'function hasher() public view returns (address)'
];
```

---

## 🔐 Security Features

- Reentrancy protection on all state-changing functions
- Access control (only owner/session key/guardian can call specific functions)
- Spending limits enforced for session keys
- Time-based expiration for session keys
- 24-hour delay for guardian recovery
- Nullifier system prevents double-spending in private transfers

---

## 📝 Common Use Cases

### 1. Create Wallet for User

```typescript
async function createWallet(ownerAddress: string, email: string) {
  const salt = ethers.keccak256(ethers.toUtf8Bytes(email));
  const predictedAddress = await factory.getAddress(ownerAddress, salt);
  
  // Check if already deployed
  const code = await provider.getCode(predictedAddress);
  if (code !== '0x') {
    return predictedAddress; // Already exists
  }
  
  // Deploy
  const signer = new ethers.Wallet(privateKey, provider);
  const factoryWithSigner = factory.connect(signer);
  const tx = await factoryWithSigner.createAccount(ownerAddress, salt);
  await tx.wait();
  
  return predictedAddress;
}
```

### 2. Grant dApp Session Key Access

```typescript
async function grantSessionKey(
  accountAddress: string,
  sessionKeyAddress: string,
  daysValid: number,
  spendingLimit: string
) {
  const account = new ethers.Contract(accountAddress, ACCOUNT_ABI, signer);
  
  const validUntil = Math.floor(Date.now() / 1000) + (daysValid * 24 * 60 * 60);
  const limit = ethers.parseEther(spendingLimit);
  
  const tx = await account.addSessionKey(sessionKeyAddress, validUntil, limit);
  await tx.wait();
}
```

### 3. Send Private Transfer

```typescript
async function sendPrivateTransfer(
  recipient: string,
  amount: string,
  tokenAddress: string
) {
  const verifier = new ethers.Contract(VERIFIER_ADDRESS, VERIFIER_ABI, provider);
  const token = new ethers.Contract(tokenAddress, TOKEN_ABI, signer);
  
  // Generate commitment inputs
  const amountBigInt = ethers.parseEther(amount);
  const blinding = ethers.randomBytes(32);
  const recipientBytes = ethers.zeroPadValue(recipient, 32);
  const nonce = ethers.randomBytes(32);
  
  const inputs: [string, string, string, string] = [
    ethers.zeroPadValue(ethers.toBeHex(amountBigInt), 32),
    ethers.hexlify(blinding),
    recipientBytes,
    ethers.hexlify(nonce)
  ];
  
  // Get commitment
  const commitment = await verifier.verifyCommitment(inputs);
  
  // Create nullifier
  const nullifier = ethers.keccak256(
    ethers.concat([commitment, ethers.toUtf8Bytes('nullifier')])
  );
  
  // Encode proof
  const proof = ethers.AbiCoder.defaultAbiCoder().encode(
    ['bytes32[4]'],
    [inputs]
  );
  
  // Execute
  const tx = await token.privateTransfer(
    commitment,
    nullifier,
    amountBigInt,
    proof
  );
  await tx.wait();
  
  return { commitment, nullifier };
}
```

---

## 🧪 Testing

```bash
# Run all tests
forge test --match-path "test/veil-wallet/**" -vvv

# Test specific contract
forge test --match-path "test/veil-wallet/SmartAccount.t.sol" -vvv
```

---

## 🚀 Development

```bash
# Compile
forge build

# Deploy
forge script script/deployVeilWallet.s.sol:DeployVeilWalletContracts \
  --rpc-url $MANTLE_TESTNET_RPC_URL \
  --broadcast \
  --verify
```

---
