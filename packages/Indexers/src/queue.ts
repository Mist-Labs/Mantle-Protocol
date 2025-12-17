import { Queue, Worker } from 'bullmq';
import { config } from './config';
import { GoldskyWebhookPayload, RelayerEventPayload } from './types';
import { 
  handleIntentCreated, 
  handleIntentFilled, 
  handleIntentRefunded, 
  handleWithdrawalClaimed, 
  handleRootSynced 
} from './handler';
import { sendToRelayer } from './relayer';

const connection = {
  host: config.redis.host,
  port: config.redis.port,
  password: config.redis.password,
};

export const eventQueue = new Queue('shadowswap-events', {
  connection,
  defaultJobOptions: {
    attempts: 3,
    backoff: {
      type: 'exponential',
      delay: 2000,
    },
  },
});

export async function initQueue() {
  const worker = new Worker('shadowswap-events', async (job) => {
    const { payload, idempotencyKey } = job.data as {
      payload: GoldskyWebhookPayload;
      idempotencyKey: string;
    };
    
    console.log(`🔄 Processing: ${payload.event.name} (${idempotencyKey})`);
    
    let relayerPayload: RelayerEventPayload;
    
    switch (payload.event.name) {
      case 'intent_created':
        relayerPayload = await handleIntentCreated(payload);
        break;
      case 'intent_filled':
        relayerPayload = await handleIntentFilled(payload);
        break;
      case 'intent_refunded':
        relayerPayload = await handleIntentRefunded(payload);
        break;
      case 'withdrawal_claimed':
        relayerPayload = await handleWithdrawalClaimed(payload);
        break;
      case 'root_synced':
        relayerPayload = await handleRootSynced(payload);
        break;
      default:
        console.log(`⚠️  Unknown event: ${payload.event.name}`);
        return { status: 'ignored' };
    }
    
    await sendToRelayer(relayerPayload);
    
    return { status: 'success', idempotencyKey };
  }, {
    connection,
    concurrency: 5,
  });
  
  worker.on('completed', (job) => {
    console.log(`✅ Completed: ${job.id}`);
  });
  
  worker.on('failed', (job, err) => {
    console.error(`❌ Failed: ${job?.id}`, err.message);
  });
  
  console.log('✅ Queue initialized');
}