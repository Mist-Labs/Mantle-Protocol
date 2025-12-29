import { Queue, Worker } from "bullmq";
import { config } from "./config";
import { GoldskyWebhookPayload } from "./types";
import {
  handleIntentCreated,
  handleIntentRegistered,
  handleIntentFilled,
  handleIntentMarkedFilled,
  handleIntentRefunded,
  handleWithdrawalClaimed,
  handleRootSynced,
} from "./handler";
import crypto from "crypto";
import axios from "axios";

const connection = {
  host: config.redis.host,
  port: config.redis.port,
  password: config.redis.password,
};

export const eventQueue = new Queue("goldsky-events", { connection });

function generateHMACSignature(payload: any): {
  signature: string;
  timestamp: string;
} {
  const requestBody = JSON.stringify(payload);
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const message = timestamp + requestBody;

  const signature = crypto
    .createHmac("sha256", config.hmacSecret)
    .update(message)
    .digest("hex");

  return { signature, timestamp };
}

async function forwardToRelayer(payload: any): Promise<void> {
  const { signature, timestamp } = generateHMACSignature(payload);

  try {
    const response = await axios.post(
      `${config.relayerBaseUrl}/indexer/event`,
      payload,
      {
        headers: {
          "Content-Type": "application/json",
          "x-signature": signature,
          "x-timestamp": timestamp,
        },
        timeout: 30000,
      }
    );

    console.log(`✅ Forwarded to relayer: ${response.status}`);
  } catch (error: any) {
    console.error("❌ Failed to forward to relayer:", error.message);
    throw error;
  }
}

const worker = new Worker(
  "goldsky-events",
  async (job) => {
    const { payload, idempotencyKey } = job.data as {
      payload: GoldskyWebhookPayload;
      idempotencyKey: string;
    };

    console.log(`⚙️  Processing: ${payload.entity} | ID: ${idempotencyKey}`);

    try {
      const entity = payload.entity;
      let transformedPayload;

      // Route to appropriate handler based on entity type
      switch (entity) {
        case "intent_created":
          transformedPayload = await handleIntentCreated(payload);
          break;
        case "intent_registered":
          transformedPayload = await handleIntentRegistered(payload);
          break;
        case "intent_filled":
          transformedPayload = await handleIntentFilled(payload);
          break;
        case "intent_marked_filled":
          transformedPayload = await handleIntentMarkedFilled(payload);
          break;
        case "intent_refunded":
          transformedPayload = await handleIntentRefunded(payload);
          break;
        case "withdrawal_claimed":
          transformedPayload = await handleWithdrawalClaimed(payload);
          break;
        case "root_synced":
          transformedPayload = await handleRootSynced(payload);
          break;
        default:
          console.warn(`⚠️  Unknown entity type: ${entity}`);
          return;
      }

      // Forward to relayer
      await forwardToRelayer(transformedPayload);

      console.log(`✅ Completed: ${entity}`);
    } catch (error) {
      console.error(`❌ Processing failed:`, error);
      throw error;
    }
  },
  {
    connection,
    concurrency: 5,
    limiter: {
      max: 10,
      duration: 1000,
    },
  }
);

worker.on("completed", (job) => {
  console.log(`✅ Job ${job.id} completed`);
});

worker.on("failed", (job, err) => {
  console.error(`❌ Job ${job?.id} failed:`, err.message);
});

export async function initQueue() {
  await eventQueue.waitUntilReady();
  console.log("✅ Queue initialized");
}
