import express, { NextFunction, Request, Response } from "express";
import { config } from "./config";
import { GoldskyWebhookPayload } from "./types";
import { eventQueue, initQueue } from "./queue";
import { deriveChainId } from "./utils";

const SUPPORTED_ENTITIES = [
  "intent_created",
  "intent_registered",
  "intent_filled",
  "intent_settled",
  "intent_refunded",
  "withdrawal_claimed",
  "root_synced",
  "fill_root_synced",
  "commitment_root_synced"
];

const SUPPORTED_CHAIN_IDS = ["5003", "11155111"]; // As strings to match Goldsky format

const app = express();

app.use(express.json());

app.use(
  "/webhook",
  (req: Request, _res: Response, next: NextFunction): void => {
    const secret = req.headers["goldsky-webhook-secret"];

    if (!secret || secret !== config.goldskyWebhookSecret) {
      console.warn("⚠️  Unauthorized webhook attempt");
      _res.status(401).json({ error: "Unauthorized" });
      return;
    }

    next();
  }
);

app.post("/webhook", async (req: Request, res: Response): Promise<void> => {
  try {
    const payload: GoldskyWebhookPayload = req.body;

    // DEBUG: Log full payload
    console.log("📦 Full Goldsky Payload:", JSON.stringify(payload, null, 2));

    // Validate Goldsky payload structure
    if (!payload.data?.new || !payload.entity) {
      console.error("❌ Invalid Goldsky payload structure");
      res.status(400).json({
        error: "Invalid webhook payload",
        details: "Missing data.new or entity field",
      });
      return;
    }

    const { entity, data } = payload;
    const eventData = data.new;

    // Derive chain ID using shared utility
    const chainId = deriveChainId(payload);

    // DEBUG: Log extracted data
    console.log("📊 Entity:", entity);
    console.log("📊 Chain ID (derived):", chainId);
    console.log("📊 Event data:", JSON.stringify(eventData, null, 2));

    // Validate entity type
    if (!SUPPORTED_ENTITIES.includes(entity)) {
      console.log(`⚠️  Unsupported entity: ${entity}`);
      res.status(200).json({ status: "ignored", reason: "unsupported_entity" });
      return;
    }

    // Validate chain using derived chainId
    if (!chainId || !SUPPORTED_CHAIN_IDS.includes(chainId)) {
      console.log(`⚠️  Unsupported chain: ${chainId}`);
      res.status(200).json({ status: "ignored", reason: "unsupported_chain" });
      return;
    }

    const idempotencyKey = eventData.id; // Goldsky provides unique ID

    console.log(
      `📡 Received: ${entity} | Chain: ${chainId} | Tx: ${eventData.transaction_hash}`
    );

    // Respond immediately to Goldsky
    res.status(200).json({
      status: "received",
      idempotency_key: idempotencyKey,
    });

    // Queue for processing
    await eventQueue.add(
      "process-event",
      {
        payload,
        idempotencyKey,
        receivedAt: Date.now(),
      },
      {
        jobId: idempotencyKey,
        attempts: 3,
        backoff: {
          type: "exponential",
          delay: 2000,
        },
        removeOnComplete: true,
        removeOnFail: false,
      }
    );
  } catch (error) {
    console.error("❌ Webhook error:", error);
    res.status(200).json({ status: "queued_with_error" });
  }
});

app.get("/health", (_req: Request, res: Response): void => {
  res.status(200).json({
    status: "healthy",
    timestamp: Date.now(),
  });
});

app.get("/queue/stats", async (_req: Request, res: Response): Promise<void> => {
  const counts = await eventQueue.getJobCounts();
  res.json(counts);
});

process.on("SIGTERM", async () => {
  console.log("🛑 Shutting down gracefully...");
  await eventQueue.close();
  process.exit(0);
});

const startServer = async () => {
  await initQueue();

  app.listen(config.port, () => {
    console.log(`🚀 Indexer running on port ${config.port}`);
    console.log(`   Environment: ${config.nodeEnv}`);
    console.log(`   Relayer: ${config.relayerBaseUrl}`);
  });
};

startServer();
