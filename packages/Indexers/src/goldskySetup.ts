import axios from "axios";
import dotenv from "dotenv";

dotenv.config();

const GOLDSKY_API_URL = "https://api.goldsky.com/api/v1";

interface ContractConfig {
  address: string;
  events: string[];
  startBlock?: number | "latest";
}

interface WebhookPipelineConfig {
  name: string;
  chainId: number;
  contracts: ContractConfig[];
  webhookUrl: string;
  webhookSecret: string;
}

async function createWebhookPipeline(config: WebhookPipelineConfig) {
  const apiKey = process.env.GOLDSKY_API_KEY;

  if (!apiKey) {
    throw new Error(
      "GOLDSKY_API_KEY not found. Get it from goldsky.com dashboard."
    );
  }

  console.log(`\n🔧 Creating Goldsky webhook pipeline: ${config.name}`);
  console.log(`   Chain ID: ${config.chainId}`);
  console.log(`   Webhook URL: ${config.webhookUrl}`);
  console.log(`   Contracts: ${config.contracts.length}`);

  try {
    const response = await axios.post(
      `${GOLDSKY_API_URL}/pipelines/webhook`,
      {
        name: config.name,
        chain_id: config.chainId,
        webhook_url: config.webhookUrl,
        webhook_secret: config.webhookSecret,
        contracts: config.contracts.map((c) => ({
          address: c.address.toLowerCase(),
          events: c.events,
          start_block: c.startBlock || "latest",
        })),
      },
      {
        headers: {
          Authorization: `Bearer ${apiKey}`,
          "Content-Type": "application/json",
        },
      }
    );

    console.log("✅ Pipeline created successfully!");
    console.log("   Pipeline ID:", response.data.id);
    console.log("   Status:", response.data.status);

    return response.data;
  } catch (error: any) {
    console.error("❌ Failed to create webhook pipeline");
    if (error.response) {
      console.error("   Status:", error.response.status);
      console.error("   Error:", JSON.stringify(error.response.data, null, 2));
    } else {
      console.error("   Error:", error.message);
    }
    throw error;
  }
}

async function setupPipelines() {
  const webhookUrl = process.env.WEBHOOK_URL;
  const webhookSecret = process.env.GOLDSKY_WEBHOOK_SECRET;

  if (!webhookUrl || !webhookSecret) {
    throw new Error("Missing WEBHOOK_URL or GOLDSKY_WEBHOOK_SECRET in .env");
  }

  // Mantle Pipeline
  const mantlePipeline: WebhookPipelineConfig = {
    name: "shadowswap-mantle",
    chainId: parseInt(process.env.MANTLE_CHAIN_ID || "5000"),
    webhookUrl,
    webhookSecret,
    contracts: [
      {
        address: process.env.MANTLE_INTENT_POOL_ADDRESS!,
        events: ["IntentCreated", "IntentFilled", "IntentRefunded"],
        startBlock: "latest",
      },
      {
        address: process.env.MANTLE_SETTLEMENT_ADDRESS!,
        events: ["WithdrawalClaimed", "IntentRegistered", "RootSynced"],
        startBlock: "latest",
      },
    ],
  };

  // Ethereum Pipeline
  const ethereumPipeline: WebhookPipelineConfig = {
    name: "shadowswap-ethereum",
    chainId: parseInt(process.env.ETHEREUM_CHAIN_ID || "1"),
    webhookUrl,
    webhookSecret,
    contracts: [
      {
        address: process.env.ETHEREUM_INTENT_POOL_ADDRESS!,
        events: ["IntentCreated", "IntentFilled", "IntentRefunded"],
        startBlock: "latest",
      },
      {
        address: process.env.ETHEREUM_SETTLEMENT_ADDRESS!,
        events: ["WithdrawalClaimed", "IntentRegistered", "RootSynced"],
        startBlock: "latest",
      },
    ],
  };

  console.log("\n🚀 Setting up Goldsky webhook pipelines...\n");

  try {
    await createWebhookPipeline(mantlePipeline);
    console.log("\n---\n");
    await createWebhookPipeline(ethereumPipeline);

    console.log("\n✅ All pipelines created successfully!");
    console.log("\n📝 Next steps:");
    console.log("   1. Start your indexer: npm run dev");
    console.log("   2. Monitor webhooks in Goldsky dashboard");
    console.log("   3. Check /queue/stats endpoint for processing status");
  } catch (error) {
    console.error("\n❌ Setup failed. Please check your configuration.");
    process.exit(1);
  }
}

setupPipelines();

// Run: npx ts-node goldsky-setup.ts
