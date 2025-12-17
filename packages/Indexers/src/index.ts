import express, { NextFunction, Request, Response } from "express";
import { config } from "./config";
import { GoldskyWebhookPayload } from "./types";
import { eventQueue, initQueue } from "./queue";

const SUPPORTED_EVENTS = [
  "intent_created",
  "intent_filled",
  "intent_refunded",
  "withdrawal_claimed",
  "root_synced",
];

const SUPPORTED_CHAINS = [5003, 11155111];

const app = express();

app.use(express.json());

app.use("/webhook", (req: Request, res: Response, next: NextFunction): void => {
  const secret = req.headers["goldsky-webhook-secret"];

  if (!secret || secret !== config.goldskyWebhookSecret) {
    console.warn("⚠️  Unauthorized webhook attempt");
    res.status(401).json({ error: "Unauthorized" });
    return;
  }

  next();
});

app.post("/webhook", async (req: Request, res: Response): Promise<void> => {
  try {
    const payload: GoldskyWebhookPayload = req.body;

    if (!payload.event || !payload.chainId) {
      res.status(400).json({ error: "Invalid webhook payload" });
      return;
    }

    if (!SUPPORTED_EVENTS.includes(payload.event.name)) {
      console.log(`⚠️  Unsupported event: ${payload.event.name}`);
      res.status(200).json({ status: "ignored" });
      return;
    }

    if (!SUPPORTED_CHAINS.includes(payload.chainId)) {
      console.log(`⚠️  Unsupported chain: ${payload.chainId}`);
      res.status(200).json({ status: "ignored" });
      return;
    }

    const {
      name: eventName,
      transactionHash: txHash,
      logIndex,
    } = payload.event;
    const idempotencyKey = `${txHash}-${logIndex}`;

    console.log(
      `📡 Received: ${eventName} | Chain: ${payload.chainId} | Tx: ${txHash}`
    );

    res.status(200).json({
      status: "received",
      idempotency_key: idempotencyKey,
    });

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
