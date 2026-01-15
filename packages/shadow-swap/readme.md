# Shadow-swap Relayer

The Shadow-swap Relayer is the core off-chain infrastructure component that coordinates privacy-preserving cross-chain asset transfers between Ethereum and Mantle networks. Built with Rust for performance and reliability, the relayer monitors cross-chain intents, generates merkle proofs, coordinates solver settlement, executes privacy-preserving withdrawals, and handles intent expiration refunds.

## Overview

Shadow-swap is a privacy-preserving intent-based cross-chain bridge that combines zero-knowledge cryptography with competitive solver markets. The relayer serves as the trusted coordinator that:

- **Monitors cross-chain intents** - Listens to IntentCreated events on both chains
- **Generates merkle proofs** - Maintains off-chain commitment trees and generates proofs for verification
- **Registers intents** - Submits intents to destination chain Settlement contracts for solver filling
- **Executes claims** - Retrieves user secrets securely and executes privacy-preserving withdrawals
- **Handles refunds** - Automatically triggers refunds when intents expire unfilled
- **Synchronizes state** - Keeps merkle roots synchronized bidirectionally between chains

## Features

### Core Functionality
- **Event-driven architecture** - WebSocket subscriptions for real-time blockchain event monitoring
- **Off-chain merkle trees** - Maintains commitment trees in PostgreSQL for gas efficiency
- **Canonical hashing** - Ensures cross-chain proof compatibility with deterministic operations
- **Just-in-time secret retrieval** - Fetches and decrypts user secrets only when needed, purges immediately after use
- **Automatic failover** - Reconnection logic with exponential backoff for provider stability
- **Comprehensive monitoring** - Prometheus-compatible metrics and structured logging

### Security
- **ECIES encryption** - User secrets encrypted at rest, relayer-only decryption
- **ReentrancyGuard patterns** - Protected against common exploit vectors
- **Balance validation** - Checks before transfers prevent failed transactions
- **Intent parameter validation** - Ensures commitment integrity throughout lifecycle

### Production-Grade Infrastructure
- **PostgreSQL** - Persistent state management with automatic schema migrations
- **Health monitoring** - Automated health checks and graceful degradation
- **Metrics tracking** - Real-time performance and system health metrics
- **Error handling** - Comprehensive error recovery with circuit breakers

## Architecture

The relayer is organized into specialized coordinator services:

```
shadow-swap/
├── src/
│   ├── coordinators/
│   │   ├── merkle_tree_manager.rs    # Manages commitment trees
│   │   ├── root_sync_coordinator.rs  # Synchronizes merkle roots
│   │   ├── bridge_coordinator.rs     # Coordinates bridge lifecycle
│   │   └── secret_monitor.rs         # Handles user secret management
│   ├── routes/
│   │   └── api.rs                    # REST API endpoints
│   ├── models/                       # Database models
│   ├── utils/                        # Helper utilities
│   └── main.rs                       # Application entry point
├── migrations/                       # Database migrations
└── Cargo.toml                        # Dependencies
```

## Prerequisites

- **Rust** - Latest stable version (no specific version requirement, managed by Cargo)
- **PostgreSQL** - Database for persistent state (see [Installation](#installation))
- **RPC Providers** - Ethereum and Mantle RPC endpoints (HTTP + WebSocket)
- **Private Keys** - Funded relayer wallet for gas fees

All Rust dependencies are managed in `Cargo.toml` and will be installed automatically.

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/Mist-Labs/Mantle-Protocol.git
cd Mantle-Protocol/packages/shadow-swap
```

### 2. Set Up PostgreSQL

Install PostgreSQL if not already installed:

```bash
# macOS
brew install postgresql@14
brew services start postgresql@14

# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib
sudo systemctl start postgresql

# Create database
createdb shadow-swap-EVM
```

### 3. Configure Environment

Copy the example environment file and fill in your values:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:
- Set `DATABASE_URL` with your PostgreSQL credentials
- Add your RPC endpoints (HTTP and WebSocket)
- Configure private keys for relayer operations
- Set deployed contract addresses
- Configure chain IDs (testnet or mainnet)

**Important**: Never commit your `.env` file. Keep private keys secure.

### 4. Build the Project

```bash
cargo build --release
```

Database migrations will run automatically on first startup.

## Configuration

### Environment Variables

Key configuration options in `.env`:

| Variable | Description | Example |
|----------|-------------|---------|
| `HOST` | Server bind address | `0.0.0.0` |
| `PORT` | Server port | `8080` |
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@localhost:5432/db` |
| `ETHEREUM_RPC_URL` | Ethereum RPC endpoint | `https://ethereum-sepolia-rpc.publicnode.com` |
| `ETHEREUM_WS_URL` | Ethereum WebSocket endpoint | `wss://ethereum-sepolia-rpc.publicnode.com` |
| `MANTLE_RPC_URL` | Mantle RPC endpoint | `https://rpc.sepolia.mantle.xyz` |
| `MANTLE_WS_URL` | Mantle WebSocket endpoint | `wss://mantle-sepolia.drpc.org` |
| `RELAYER_PRIVATE_KEY` | Private key for relayer operations | `0x...` |
| `RELAYER_ADDRESS` | Wallet address for relayer operations | `0x...` |
| `FEE_COLLECTOR` | Wallet address for collecting bridge fees | `0x...` |
| `SYNC_ON_STARTUP` | Sync historical events on startup | `false` |
| `ETHEREUM_SYNC_FROM_BLOCK` | Block to start syncing from | `10007553` |
| `MANTLE_SYNC_FROM_BLOCK` | Block to start syncing from | `33197983` |
| `RPC_BATCH_SIZE` | Batch size for RPC queries | `2000` |
| `RPC_DELAY_MS` | Delay between RPC batches (ms) | `300` |

### Contract Addresses

Ensure the following contract addresses are configured correctly:

- `ETHEREUM_INTENT_POOL_ADDRESS` - IntentPool contract on Ethereum
- `ETHEREUM_SETTLEMENT_ADDRESS` - Settlement contract on Ethereum
- `MANTLE_INTENT_POOL_ADDRESS` - IntentPool contract on Mantle
- `MANTLE_SETTLEMENT_ADDRESS` - Settlement contract on Mantle

## Running the Relayer

### Development Mode

```bash
cargo run
```

### Production Mode

```bash
cargo run --release
```

### Debug Mode

For verbose logging during development:

```bash
RUST_LOG=debug cargo run
```

Or set specific log levels:

```bash
RUST_LOG=mantle_bridge=debug,actix_web=info cargo run
```

### Running as a Service

For production deployments, consider using systemd or Docker:

```bash
# Build release binary
cargo build --release

# Binary location
./target/release/shadow-swap
```

## API Endpoints

The relayer exposes a REST API on `http://localhost:8080/api/v1`:

### Health & Monitoring

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/` | GET | Root endpoint |
| `/api/v1/health` | GET | Health check - returns relayer status |
| `/api/v1/metrics` | GET | Prometheus-compatible metrics |
| `/api/v1/stats` | GET | System statistics and performance data |

### Bridge Operations

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/bridge/initiate` | POST | Initiate a new bridge transaction |
| `/api/v1/intents/:id` | GET | Get intent status by ID |
| `/api/v1/intents` | GET | List all intents (with pagination) |

### Price & Conversion

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/price` | GET | Get token price |
| `/api/v1/prices` | GET | Get all token prices |
| `/api/v1/convert` | POST | Convert amount between tokens |

### Internal (Indexer)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/indexer/event` | POST | Receive indexer events |

### Example: Check Health

```bash
curl http://localhost:8080/api/v1/health
```

Response:
```json
{
  "status": "healthy",
  "database": "connected",
  "ethereum_provider": "connected",
  "mantle_provider": "connected"
}
```

### Example: Get Intent Status

```bash
curl http://localhost:8080/api/v1/intents/<intent_id>
```

## Monitoring

### Health Checks

Monitor relayer health via the `/health` endpoint:

```bash
# Basic health check
curl http://localhost:8080/api/v1/health

# With monitoring tool
watch -n 5 'curl -s http://localhost:8080/api/v1/health | jq'
```

### Metrics

Access Prometheus-compatible metrics:

```bash
curl http://localhost:8080/api/v1/metrics
```

Key metrics include:
- Intent processing rates
- Merkle proof generation time
- Transaction success/failure rates
- Provider connection status
- Database query performance

### Logs

The relayer uses structured logging. Configure verbosity with `RUST_LOG`:

```bash
# Info level (default)
RUST_LOG=info cargo run

# Debug level (verbose)
RUST_LOG=debug cargo run

# Module-specific
RUST_LOG=shadow_swap=debug,actix_web=info cargo run
```

## Database

### Schema Migrations

Migrations run automatically on startup. The relayer will:
1. Check current database schema version
2. Apply pending migrations if needed
3. Validate schema integrity

### Manual Migration

If needed, you can run migrations manually:

```bash
diesel migrate run
```

### Backup

Regular database backups are recommended:

```bash
pg_dump shadow-swap-EVM > backup-$(date +%Y%m%d).sql
```

## Debugging

### Common Issues

**1. WebSocket Connection Failures**
- Check `ETHEREUM_WS_URL` and `MANTLE_WS_URL` are correct
- Verify RPC provider supports WebSocket connections
- Enable debug logging: `RUST_LOG=debug cargo run`

**2. Transaction Reverts**
- Ensure relayer wallet has sufficient ETH/MNT for gas
- Verify contract addresses match deployed contracts
- Check chain IDs are correct for target network

**3. Database Connection Issues**
- Verify PostgreSQL is running: `pg_isready`
- Check `DATABASE_URL` credentials
- Ensure database exists: `psql -l | grep shadow-swap`

**4. Merkle Proof Verification Failures**
- Confirm `SYNC_ON_STARTUP` is enabled for first run
- Check block sync numbers are before contract deployment
- Verify canonical hashing implementation matches contracts

### Debug Mode

Run with full debug output:

```bash
RUST_LOG=debug cargo run
```

This will show:
- All RPC requests/responses
- Event processing details
- Merkle tree operations
- Transaction submissions
- Error stack traces

### Testing

Run the test suite:

```bash
cargo test
```

With output:
```bash
cargo test -- --nocapture
```

## Security Considerations

### Private Key Management
- **Never commit private keys** to version control
- Use hardware wallets or secure key management systems in production
- Rotate keys periodically
- Limit relayer wallet permissions to minimum required

### Secret Handling
- User secrets are ECIES encrypted at rest
- Secrets are decrypted just-in-time for claim execution
- Memory is purged immediately after use
- 24-hour automatic expiry on stored secrets
- Comprehensive access logging for audit trails

### Network Security
- Use HTTPS/WSS for RPC connections in production
- Implement rate limiting on API endpoints
- Enable firewall rules to restrict access
- Monitor for unusual transaction patterns

### Database Security
- Use strong PostgreSQL credentials
- Enable SSL for database connections
- Regular backups with encryption
- Restrict database access to relayer only

## Related Services

This relayer is part of the Shadow-swap monorepo:

```
Mantle-Protocol/
├── packages/
│   ├── shadow-swap/           # This relayer
│   ├── solver/                # Solver service (separate)
│   ├── contracts/             # Smart contracts
│   └── frontend/              # Web interface
```

## Contributing

When contributing to the relayer:

1. Follow Rust naming conventions
2. Add tests for new functionality
3. Update documentation for API changes
4. Run `cargo fmt` before committing
5. Ensure `cargo clippy` passes
6. Test with both testnet and mainnet configurations

## License

[License information]

## Support

For issues, questions, or contributions:
- **GitHub Issues**: [Repository Issues](https://github.com/Mist-Labs/Mantle-Protocol/issues)
- **Email**: ebounce500@gmail.com

---

**Shadow-swap Relayer** - Privacy-preserving cross-chain infrastructure