import axios, { AxiosError } from "axios";
import { config } from "./config";
import { RelayerEventPayload } from "./types";
import { createHmacSignature } from "./utils";

const MAX_RETRIES = 3;
const RETRY_DELAY = 1000;

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function sendToRelayer(
  payload: RelayerEventPayload,
  attempt: number = 1
): Promise<void> {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const payloadString = JSON.stringify(payload);
  const message = timestamp + payloadString;
  const signature = createHmacSignature(message);

  const url = `${config.relayerBaseUrl}/indexer/event`;

  try {
    const response = await axios.post(url, payload, {
      headers: {
        "Content-Type": "application/json",
        "X-Signature": signature,
        "X-Timestamp": timestamp,
        "X-Idempotency-Key": `${payload.transaction_hash}-${payload.log_index}`,
      },
      timeout: 10000,
      validateStatus: (status) => status < 500,
    });

    if (response.status === 200) {
      console.log(
        `✅ Sent: ${payload.event_type} | ${payload.transaction_hash}`
      );
      return;
    }

    if (response.status >= 400 && response.status < 500) {
      console.error(
        `❌ Client error (${response.status}): ${payload.event_type}`
      );
      console.error(`   ${response.data?.message || "Unknown error"}`);
      return;
    }

    throw new Error(`Server error: ${response.status}`);
  } catch (error) {
    const axiosError = error as AxiosError;

    if (axiosError.code === "ECONNABORTED" || axiosError.code === "ENOTFOUND") {
      console.error(
        `⚠️  Network error (${attempt}/${MAX_RETRIES}): ${axiosError.message}`
      );
    } else {
      console.error(
        `❌ Send failed (${attempt}/${MAX_RETRIES}): ${payload.event_type}`
      );
    }

    if (attempt < MAX_RETRIES) {
      const delay = RETRY_DELAY * Math.pow(2, attempt - 1);
      console.log(`   Retrying in ${delay}ms...`);
      await sleep(delay);
      return sendToRelayer(payload, attempt + 1);
    }

    console.error(`❌ All retries exhausted: ${payload.transaction_hash}`);
    throw error;
  }
}
