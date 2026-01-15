# Shadow-swap Solver

The Shadow-swap Solver is an independent service that monitors registered intents on Settlement contracts and provides instant liquidity by filling them. Solvers earn 0.1% fees on deployed capital by competing to fill bridge intents quickly and efficiently.

## Overview

Solvers are the liquidity providers in Shadow-swap's intent-based architecture. Instead of massive idle liquidity pools, solvers actively deploy capital when profitable opportunities arise. The solver service:

- **Monitors IntentRegistered events** - Listens to Settlement contracts on both chains
- **Evaluates profitability** - Analyzes fees, gas costs, and risk scores for each intent
- **Fills intents** - Provides immediate liquidity on destination chain
- **Claims reimbursement** - Submits merkle proofs to source IntentPool for repayment
- **Manages balances** - Automatically tracks and manages token balances across chains

## How Solvers Make Money

**Revenue Model**:
- Earn **0.1% fee** on every intent filled
- Capital deployed only when profitable
- Potential **15%+ APY** on actively deployed capital

**Example**:
```
User bridges $10,000 USDC from Ethereum to Mantle
↓
Solver fills intent on Mantle (provides $10,000 USDC)
↓
Solver earns $10 fee (0.1%)
↓
Solver claims $10,010 USDC from Ethereum IntentPool
↓
Net profit: $10 - gas costs (~$3-5)
```

## Features

### Core Functionality
- **Real-time monitoring** - WebSocket subscriptions for instant event detection
- **Profitability analysis** - Factors in gas prices, fees, and risk scoring
- **Automatic ERC20 approvals** - Manages token approvals for Settlement contracts
- **Balance tracking** - Multi-chain balance management per token
- **Risk assessment** - Evaluates intent age, amount size, confirmation depth

### Capital Management
- **Configurable reserves** - Set minimum capital requirements per token
- **Profit thresholds** - Only fill intents meeting minimum profit margins
- **Balance monitoring** - Alerts when balances fall below thresholds
- **Gas optimization** - Dynamic gas price evaluation

### Production Features
- **HTTP server** - Metrics and health check endpoints
- **Comprehensive logging** - Structured logs for monitoring and debugging
- **Automatic reconnection** - Handles WebSocket disconnections gracefully
- **Error recovery** - Retry logic for failed transactions

**Note:** (Minimum intent amount advised for tests: $100 and above for solver to fill as profitable, as unprofitable fills are rejected).

## Prerequisites

- **Rust** - Latest stable version
- **Funded wallet** - ETH/MNT for gas + capital for filling intents
- **RPC Providers** - WebSocket endpoints for Ethereum and Mantle

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/Mist-Labs/Mantle-Protocol.git
cd Mantle-Protocol/packages/solver
```

### 2. Configure Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:
- Set `SOLVER_PRIVATE_KEY` with your funded wallet
- Set `SOLVER_ADDRESS` to match your private key
- Configure WebSocket RPC endpoints
- Verify contract addresses match deployed contracts

**Important**: Keep your private key secure. This wallet needs:
- ETH on Ethereum for gas fees
- MNT on Mantle for gas fees
- Token balances (USDC, USDT, etc.) for filling intents

### 3. Build the Project

```bash
cargo build --release
```

## Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `SOLVER_PRIVATE_KEY` | Private key for solver wallet | `0x...` |
| `SOLVER_ADDRESS` | Solver wallet address | `0xe8EeC795...` |
| `HTTP_PORT` | HTTP server port | `9000` |
| `ETHEREUM_WS_RPC` | Ethereum WebSocket endpoint | `wss://ethereum-sepolia-rpc.publicnode.com` |
| `MANTLE_WS_RPC` | Mantle WebSocket endpoint | `wss://mantle-sepolia.drpc.org` |
| `ETHEREUM_SETTLEMENT` | Settlement contract on Ethereum | `0x7CCC9864...` |
| `MANTLE_SETTLEMENT` | Settlement contract on Mantle | `0x1c4F9eB...` |
| `ETHEREUM_INTENT_POOL` | IntentPool contract on Ethereum | `0xcb46d916...` |
| `MANTLE_INTENT_POOL` | IntentPool contract on Mantle | `0x6ebcF830...` |
| `RUST_LOG` | Logging level | `solver=debug,actix_web=info` |


## Running the Solver

### Development Mode

```bash
cargo run
```

### Production Mode

```bash
cargo run --release
```

### Debug Mode

For verbose logging:

```bash
RUST_LOG=debug cargo run
```

Or with specific modules:

```bash
RUST_LOG=solver=debug,actix_web=info cargo run
```

### Running as a Service

For production deployments:

```bash
# Build release binary
cargo build --release

# Run binary
./target/release/solver
```

Consider using systemd or Docker for automatic restarts.

## Capital Requirements

### Minimum Recommended Capital

To run a competitive solver, you'll need:

**Gas Fees** (per chain):
- Ethereum: 0.5-1 ETH (~$1,000-2,000)
- Mantle: 10-20 MNT (~$10-20)

**Working Capital** (per token):
- Start: $10,000-50,000 per token
- Competitive: $100,000+ per token
- High volume: $500,000+ per token

**Total Minimum**: ~$25,000-50,000 to start
- Covers gas, initial working capital, buffer for volatility

### Capital Efficiency

- Capital only deployed when filling intents
- Reimbursement typically within 60-120 seconds
- High turnover = high returns on deployed capital
- Can start small and scale up based on volume

## Monitoring

### Health Checks

The solver exposes an HTTP server on the configured port:

```bash
# Check solver health
curl http://localhost:9000/health

# Example response
{
  "status": "healthy",
  "ethereum_balance": "1.5 ETH",
  "mantle_balance": "15.2 MNT",
  "usdc_balance_ethereum": "25000.00",
  "usdc_balance_mantle": "18000.00"
}
```

### Metrics

Access solver metrics:

```bash
curl http://localhost:9000/metrics
```

Key metrics:
- Intents monitored
- Intents filled
- Fill success rate
- Average profit per fill
- Gas costs
- Balance levels

### Logs

Monitor solver activity via logs:

```bash
# Follow logs in real-time
tail -f solver.log

# With structured logging
RUST_LOG=solver=debug cargo run 2>&1 | tee solver.log
```

## Profitability Analysis

### How Solvers Evaluate Intents

For each registered intent, the solver calculates:

```
Gross Profit = Intent Amount × 0.001 (0.1% fee)
Estimated Gas Cost = Current Gas Price × Estimated Gas Usage
Net Profit = Gross Profit - Gas Cost

If Net Profit > Min Threshold → Fill Intent (Minimum intent amount advised: $100 and above for solver to fill as profitable, as unprofitable fills are rejected).
```

### Risk Scoring

Solvers adjust profitability based on risk factors:

- **Intent Age**: Older intents = higher risk (user may have abandoned)
- **Amount Size**: Larger amounts = higher risk exposure
- **Block Confirmations**: Fewer confirmations = reorg risk
- **Gas Price Volatility**: Spikes in gas price = lower profit

### Example Profitability

**Scenario 1**: $10,000 USDC bridge
```
Fee earned: $10 (0.1%)
Gas cost: $3 (Ethereum) + $0.10 (Mantle) = $3.10
Net profit: $6.90
ROI: ~69% on 60-second deployment
Annualized: ~363,000%
```

**Scenario 2**: $100,000 USDC bridge
```
Fee earned: $100 (0.1%)
Gas cost: $3 (Ethereum) + $0.10 (Mantle) = $3.10
Net profit: $96.90
ROI: ~97% on 60-second deployment
```

**Scenario 3**: $1,000 USDC bridge (unprofitable)
```
Fee earned: $1 (0.1%)
Gas cost: $3.10
Net profit: -$2.10 (LOSS)
→ Solver skips this intent
```

## Best Practices

### Capital Management

1. **Diversify across tokens** - Don't put all capital in one asset
2. **Monitor gas prices** - Pause during extreme gas spikes
3. **Set profit thresholds** - Only fill profitable intents
4. **Maintain reserves** - Keep buffer for gas and volatility
5. **Rebalance regularly** - Move capital between chains as needed

### Risk Management

1. **Start small** - Test with minimal capital first
2. **Monitor closely** - Watch logs and metrics during first week
3. **Set alerts** - Low balance warnings, failed fills, high gas
4. **Gradual scale** - Increase capital as you gain confidence
5. **Emergency shutdown** - Have a kill switch ready

### Operational Excellence

1. **Run reliable infrastructure** - Use high-quality RPC providers
2. **Monitor 24/7** - Set up alerting for downtime
3. **Keep software updated** - Pull latest solver updates regularly
4. **Backup keys securely** - Store private keys in secure location
5. **Track performance** - Analyze fills, profits, and efficiency

## Troubleshooting

### Common Issues

**1. Intent Fill Failures**
- Check solver has sufficient token balance
- Verify ERC20 approvals are set correctly
- Ensure gas price isn't too low
- Confirm intent hasn't expired

**2. Low Profitability**
- Increase capital to fill larger intents
- Adjust profit thresholds
- Optimize gas settings
- Check for competing solvers

**3. WebSocket Disconnections**
- Use reliable RPC providers
- Implement automatic reconnection (built-in)
- Monitor provider health
- Have backup providers ready

**4. Claim Failures**
- Wait for relayer to register intent on source chain
- Verify merkle proofs are valid
- Ensure intent was actually filled
- Check gas limits

### Debug Mode

Run with full debug output:

```bash
RUST_LOG=debug cargo run
```

This shows:
- All events detected
- Profitability calculations
- Transaction submissions
- Balance updates
- Error details

## Economic Model

### Solver Competition

Multiple solvers compete to fill intents:
- **First-come, first-served** - Fastest solver wins
- **MEV-resistant** - Future: Fair ordering mechanisms
- **Market-driven** - Profitable intents get filled quickly

### Revenue Potential

Conservative estimates (assuming 50% fill rate):

| Daily Volume | Fills/Day | Revenue/Day | Monthly Revenue |
|--------------|-----------|-------------|-----------------|
| $100K | 10 × $10K | $50 | $1,500 |
| $500K | 50 × $10K | $250 | $7,500 |
| $1M | 100 × $10K | $500 | $15,000 |
| $5M | 500 × $10K | $2,500 | $75,000 |

*Note: Assumes average $10K bridge, 0.1% fee, $3 gas cost per fill*

**APY on Capital**:
- $100K capital, $1M daily volume = ~18% monthly = ~216% APY
- $500K capital, $5M daily volume = ~15% monthly = ~180% APY

## Security Considerations

### Private Key Security
- **Never commit private keys** to version control
- Use hardware wallets or HSMs in production
- Rotate keys periodically
- Implement rate limiting on transactions

### Capital Security
- Start with small amounts to test
- Monitor for unusual patterns
- Set maximum fill amounts
- Implement emergency shutdown

### Network Security
- Use WSS (WebSocket Secure) for RPC connections
- Whitelist IPs for solver operations
- Monitor for suspicious activities
- Regular security audits

## Related Services

This solver is part of the Shadow-swap monorepo:

```
Mantle-Protocol/
├── packages/
│   ├── solver/                # This solver
│   ├── shadow-swap/           # Relayer service
│   ├── contracts/             # Smart contracts
│   └── frontend/              # Web interface
```

## Contributing

When contributing to the solver:

1. Follow Rust naming conventions
2. Add tests for new functionality
3. Update documentation for changes
4. Run `cargo fmt` before committing
5. Ensure `cargo clippy` passes
6. Test profitability logic thoroughly

## FAQ

**Q: How much capital do I need to start?**  
A: Minimum $25K-50K recommended. Can start smaller to test, but profitability improves with more capital.

**Q: What's the expected return?**  
A: 15-25% APY on deployed capital, depending on volume and competition.

**Q: Do I need to run 24/7?**  
A: Yes, for maximum profitability. More uptime = more fills = more revenue.

**Q: What if I run out of tokens on one chain?**  
A: Solver will skip intents for that token until you rebalance. Monitor balances closely.

**Q: Can I run multiple solvers?**  
A: Yes, but they'll compete with each other. Better to run one well-capitalized solver.

**Q: What happens if my internet drops?**  
A: Solver will attempt to reconnect automatically. Missed intents will be filled by other solvers.

**Q: How do I know if I'm profitable?**  
A: Monitor metrics endpoint for total fills, revenue, and gas costs. Track daily P&L.

## Support

For issues, questions, or contributions:
- **GitHub Issues**: [Repository Issues](https://github.com/Mist-Labs/Mantle-Protocol/issues)
- **Email**: ebounce500@gmail.com

---

**Shadow-swap Solver** - Earn competitive yields on deployed capital while providing instant cross-chain liquidity
