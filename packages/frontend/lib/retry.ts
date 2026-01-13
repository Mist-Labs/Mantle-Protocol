/**
 * Retry Logic Utilities
 *
 * Provides configurable retry mechanisms with exponential backoff
 */

export interface RetryConfig {
  maxAttempts: number;
  initialDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
  retryableErrors?: (error: Error) => boolean;
  onRetry?: (attempt: number, error: Error) => void;
}

const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 3,
  initialDelayMs: 1000,
  maxDelayMs: 10000,
  backoffMultiplier: 2,
};

/**
 * Execute a function with exponential backoff retry
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
): Promise<T> {
  const {
    maxAttempts,
    initialDelayMs,
    maxDelayMs,
    backoffMultiplier,
    retryableErrors,
    onRetry,
  } = { ...DEFAULT_RETRY_CONFIG, ...config };

  let lastError: Error | undefined;
  let delay = initialDelayMs;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      // Check if error is retryable
      if (retryableErrors && !retryableErrors(lastError)) {
        throw lastError;
      }

      // Don't retry on last attempt
      if (attempt === maxAttempts) {
        throw lastError;
      }

      // Call retry callback
      if (onRetry) {
        onRetry(attempt, lastError);
      }

      // Wait with exponential backoff
      await sleep(delay);
      delay = Math.min(delay * backoffMultiplier, maxDelayMs);
    }
  }

  throw lastError;
}

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Check if an error is network-related and should be retried
 */
export function isNetworkError(error: Error): boolean {
  const message = error.message.toLowerCase();
  return (
    message.includes("network") ||
    message.includes("timeout") ||
    message.includes("fetch") ||
    message.includes("econnreset") ||
    message.includes("enotfound") ||
    message.includes("connection") ||
    message.includes("502") ||
    message.includes("503") ||
    message.includes("504")
  );
}

/**
 * Check if an error is RPC-related and should be retried
 */
export function isRPCError(error: Error): boolean {
  const message = error.message.toLowerCase();
  return (
    message.includes("rpc") ||
    message.includes("jsonrpc") ||
    message.includes("intrinsic gas too low") ||
    message.includes("nonce too low") ||
    message.includes("replacement transaction underpriced")
  );
}

/**
 * Retry specifically for API calls
 */
export async function retryAPICall<T>(
  fn: () => Promise<T>,
  maxAttempts: number = 3
): Promise<T> {
  return withRetry(fn, {
    maxAttempts,
    initialDelayMs: 1000,
    maxDelayMs: 5000,
    backoffMultiplier: 2,
    retryableErrors: (error) => {
      // Retry on network errors and 5xx status codes
      if (isNetworkError(error)) return true;

      // Check for HTTP status code in error message
      const status = parseInt(error.message.match(/\d{3}/)?.[0] || "0");
      return status >= 500 && status < 600;
    },
    onRetry: (attempt, error) => {
      console.warn(`API call failed (attempt ${attempt}):`, error.message);
    },
  });
}

/**
 * Retry specifically for blockchain RPC calls
 */
export async function retryRPCCall<T>(
  fn: () => Promise<T>,
  maxAttempts: number = 5
): Promise<T> {
  return withRetry(fn, {
    maxAttempts,
    initialDelayMs: 500,
    maxDelayMs: 3000,
    backoffMultiplier: 1.5,
    retryableErrors: (error) => {
      return isNetworkError(error) || isRPCError(error);
    },
    onRetry: (attempt, error) => {
      console.warn(`RPC call failed (attempt ${attempt}):`, error.message);
    },
  });
}

/**
 * Retry with jitter to avoid thundering herd
 */
export async function withJitter<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
): Promise<T> {
  return withRetry(fn, {
    ...config,
    initialDelayMs: (config.initialDelayMs || 1000) * (0.5 + Math.random() * 0.5),
  });
}
