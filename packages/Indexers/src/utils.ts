import crypto from 'crypto';
import { config } from './config';
import { Chain } from './types';

export function createHmacSignature(payload: string): string {
  return crypto
    .createHmac('sha256', config.hmacSecret)
    .update(payload)
    .digest('hex');
}

export function getChainName(chainId: number): Chain {
  if (chainId === config.chains.mantle.chainId) return Chain.Mantle;
  if (chainId === config.chains.ethereum.chainId) return Chain.Ethereum;
  throw new Error(`Unknown chain ID: ${chainId}`);
}

export function getContractType(chainId: number, address: string): 'intent_pool' | 'settlement' {
  const normalizedAddress = address.toLowerCase();
  const chain = getChainName(chainId);
  
  if (chain === Chain.Mantle) {
    if (normalizedAddress === config.chains.mantle.intentPoolAddress) return 'intent_pool';
    if (normalizedAddress === config.chains.mantle.settlementAddress) return 'settlement';
  } else {
    if (normalizedAddress === config.chains.ethereum.intentPoolAddress) return 'intent_pool';
    if (normalizedAddress === config.chains.ethereum.settlementAddress) return 'settlement';
  }
  
  throw new Error(`Unknown contract address: ${address} on chain ${chainId}`);
}

export function formatEventData(args: Record<string, any>): Record<string, any> {
  const formatted: Record<string, any> = {};
  
  for (const [key, value] of Object.entries(args)) {
    if (typeof value === 'bigint') {
      formatted[key] = value.toString();
    } else if (typeof value === 'object' && value !== null && 'type' in value && value.type === 'BigNumber') {
      formatted[key] = value.hex || value.toString();
    } else {
      formatted[key] = value;
    }
  }
  
  return formatted;
}