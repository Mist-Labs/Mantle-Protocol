import dotenv from 'dotenv';

dotenv.config();

const requiredEnvVars = [
  'HMAC_SECRET',
  'RELAYER_BASE_URL',
  'MANTLE_CHAIN_ID',
  'ETHEREUM_CHAIN_ID',
  'MANTLE_INTENT_POOL_ADDRESS',
  'MANTLE_SETTLEMENT_ADDRESS',
  'ETHEREUM_INTENT_POOL_ADDRESS',
  'ETHEREUM_SETTLEMENT_ADDRESS',
  'GOLDSKY_WEBHOOK_SECRET',
];

for (const envVar of requiredEnvVars) {
  if (!process.env[envVar]) {
    throw new Error(`Missing required environment variable: ${envVar}`);
  }
}

export const config = {
  port: parseInt(process.env.PORT || '3000', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  hmacSecret: process.env.HMAC_SECRET!,
  relayerBaseUrl: process.env.RELAYER_BASE_URL!,
  goldskyWebhookSecret: process.env.GOLDSKY_WEBHOOK_SECRET!,

  redis: {
    host: process.env.REDIS_HOST || 'localhost',
    port: parseInt(process.env.REDIS_PORT || '6379'),
    password: process.env.REDIS_PASSWORD,
  },
  chains: {
    mantle: {
      chainId: parseInt(process.env.MANTLE_CHAIN_ID!, 10),
      intentPoolAddress: process.env.MANTLE_INTENT_POOL_ADDRESS!.toLowerCase(),
      settlementAddress: process.env.MANTLE_SETTLEMENT_ADDRESS!.toLowerCase()
    },
    ethereum: {
      chainId: parseInt(process.env.ETHEREUM_CHAIN_ID!, 10),
      intentPoolAddress: process.env.ETHEREUM_INTENT_POOL_ADDRESS!.toLowerCase(),
      settlementAddress: process.env.ETHEREUM_SETTLEMENT_ADDRESS!.toLowerCase()
    }
  }
};