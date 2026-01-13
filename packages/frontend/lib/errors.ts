/**
 * Comprehensive Error Handling System for Shadow Swap Bridge
 *
 * Provides typed errors, user-friendly messages, and recovery suggestions
 */

/**
 * Error categories for better organization
 */
export enum ErrorCategory {
  WALLET = "WALLET",
  NETWORK = "NETWORK",
  CONTRACT = "CONTRACT",
  API = "API",
  PRIVACY = "PRIVACY",
  VALIDATION = "VALIDATION",
  UNKNOWN = "UNKNOWN",
}

/**
 * Error severity levels
 */
export enum ErrorSeverity {
  INFO = "INFO",
  WARNING = "WARNING",
  ERROR = "ERROR",
  CRITICAL = "CRITICAL",
}

/**
 * Base Bridge Error class with enhanced metadata
 */
export class BridgeError extends Error {
  public readonly category: ErrorCategory;
  public readonly severity: ErrorSeverity;
  public readonly code: string;
  public readonly userMessage: string;
  public readonly technicalMessage: string;
  public readonly recoverable: boolean;
  public readonly retryable: boolean;
  public readonly suggestion?: string;
  public readonly originalError?: Error;

  constructor(config: {
    category: ErrorCategory;
    severity: ErrorSeverity;
    code: string;
    userMessage: string;
    technicalMessage: string;
    recoverable?: boolean;
    retryable?: boolean;
    suggestion?: string;
    originalError?: Error;
  }) {
    super(config.technicalMessage);
    this.name = "BridgeError";
    this.category = config.category;
    this.severity = config.severity;
    this.code = config.code;
    this.userMessage = config.userMessage;
    this.technicalMessage = config.technicalMessage;
    this.recoverable = config.recoverable ?? true;
    this.retryable = config.retryable ?? false;
    this.suggestion = config.suggestion;
    this.originalError = config.originalError;
  }
}

/**
 * Wallet-related errors
 */
export class WalletNotConnectedError extends BridgeError {
  constructor() {
    super({
      category: ErrorCategory.WALLET,
      severity: ErrorSeverity.ERROR,
      code: "WALLET_NOT_CONNECTED",
      userMessage: "Please connect your wallet to continue",
      technicalMessage: "Wallet not connected",
      recoverable: true,
      retryable: false,
      suggestion: "Click the 'Connect Wallet' button in the top right",
    });
  }
}

export class WalletSignatureRejectedError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.WALLET,
      severity: ErrorSeverity.WARNING,
      code: "SIGNATURE_REJECTED",
      userMessage: "Transaction signature was rejected",
      technicalMessage: "User rejected signature request",
      recoverable: true,
      retryable: true,
      suggestion: "Please approve the signature request in your wallet to proceed",
      originalError,
    });
  }
}

export class InsufficientBalanceError extends BridgeError {
  constructor(required: string, available: string) {
    super({
      category: ErrorCategory.WALLET,
      severity: ErrorSeverity.ERROR,
      code: "INSUFFICIENT_BALANCE",
      userMessage: `Insufficient balance. You need ${required} but only have ${available}`,
      technicalMessage: `Insufficient balance: required ${required}, available ${available}`,
      recoverable: true,
      retryable: false,
      suggestion: "Reduce the amount or add more funds to your wallet",
    });
  }
}

/**
 * Network-related errors
 */
export class WrongNetworkError extends BridgeError {
  constructor(expected: string, current: string) {
    super({
      category: ErrorCategory.NETWORK,
      severity: ErrorSeverity.ERROR,
      code: "WRONG_NETWORK",
      userMessage: `Please switch to ${expected} network`,
      technicalMessage: `Wrong network: expected ${expected}, got ${current}`,
      recoverable: true,
      retryable: false,
      suggestion: `Switch to ${expected} in your wallet or click 'Switch Network'`,
    });
  }
}

export class NetworkTimeoutError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.NETWORK,
      severity: ErrorSeverity.ERROR,
      code: "NETWORK_TIMEOUT",
      userMessage: "Network request timed out. Please try again",
      technicalMessage: "Network timeout exceeded",
      recoverable: true,
      retryable: true,
      suggestion: "Check your internet connection and try again",
      originalError,
    });
  }
}

export class RPCError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.NETWORK,
      severity: ErrorSeverity.ERROR,
      code: "RPC_ERROR",
      userMessage: "Network RPC error. Please try again",
      technicalMessage: "RPC request failed",
      recoverable: true,
      retryable: true,
      suggestion: "The network may be congested. Wait a moment and try again",
      originalError,
    });
  }
}

/**
 * Contract-related errors
 */
export class ContractExecutionError extends BridgeError {
  constructor(reason: string, originalError?: Error) {
    super({
      category: ErrorCategory.CONTRACT,
      severity: ErrorSeverity.ERROR,
      code: "CONTRACT_EXECUTION_FAILED",
      userMessage: "Transaction failed on-chain",
      technicalMessage: `Contract execution failed: ${reason}`,
      recoverable: true,
      retryable: true,
      suggestion: "The transaction may have failed due to gas or contract state. Try again",
      originalError,
    });
  }
}

export class GasEstimationError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.CONTRACT,
      severity: ErrorSeverity.ERROR,
      code: "GAS_ESTIMATION_FAILED",
      userMessage: "Unable to estimate gas fees",
      technicalMessage: "Gas estimation failed",
      recoverable: true,
      retryable: true,
      suggestion: "The transaction may fail. Check your balance and try again",
      originalError,
    });
  }
}

export class TokenApprovalError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.CONTRACT,
      severity: ErrorSeverity.ERROR,
      code: "TOKEN_APPROVAL_FAILED",
      userMessage: "Token approval failed",
      technicalMessage: "Failed to approve token for bridge contract",
      recoverable: true,
      retryable: true,
      suggestion: "Try approving the token again",
      originalError,
    });
  }
}

/**
 * API-related errors
 */
export class APIError extends BridgeError {
  constructor(statusCode: number, message: string, originalError?: Error) {
    super({
      category: ErrorCategory.API,
      severity: ErrorSeverity.ERROR,
      code: `API_ERROR_${statusCode}`,
      userMessage: "Backend service error. Please try again",
      technicalMessage: `API error ${statusCode}: ${message}`,
      recoverable: true,
      retryable: statusCode >= 500,
      suggestion: statusCode >= 500
        ? "The backend service may be down. Please try again in a few moments"
        : "There was an error processing your request",
      originalError,
    });
  }
}

export class HMACAuthError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.API,
      severity: ErrorSeverity.CRITICAL,
      code: "HMAC_AUTH_FAILED",
      userMessage: "Authentication error. Please refresh the page",
      technicalMessage: "HMAC authentication failed",
      recoverable: true,
      retryable: false,
      suggestion: "Refresh the page and try again",
      originalError,
    });
  }
}

export class IntentNotFoundError extends BridgeError {
  constructor(intentId: string) {
    super({
      category: ErrorCategory.API,
      severity: ErrorSeverity.ERROR,
      code: "INTENT_NOT_FOUND",
      userMessage: "Transaction not found",
      technicalMessage: `Intent ${intentId} not found`,
      recoverable: false,
      retryable: false,
      suggestion: "The transaction may not have been created. Try bridging again",
    });
  }
}

/**
 * Privacy-related errors
 */
export class EncryptionError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.PRIVACY,
      severity: ErrorSeverity.CRITICAL,
      code: "ENCRYPTION_FAILED",
      userMessage: "Failed to encrypt privacy parameters",
      technicalMessage: "ECIES encryption failed",
      recoverable: true,
      retryable: true,
      suggestion: "Please try again. If the problem persists, contact support",
      originalError,
    });
  }
}

export class PoseidonHashError extends BridgeError {
  constructor(originalError?: Error) {
    super({
      category: ErrorCategory.PRIVACY,
      severity: ErrorSeverity.ERROR,
      code: "POSEIDON_HASH_FAILED",
      userMessage: "Failed to generate privacy commitment",
      technicalMessage: "Poseidon hash calculation failed",
      recoverable: true,
      retryable: true,
      suggestion: "Unable to generate commitment. Please try again",
      originalError,
    });
  }
}

/**
 * Validation errors
 */
export class InvalidAmountError extends BridgeError {
  constructor(reason: string) {
    super({
      category: ErrorCategory.VALIDATION,
      severity: ErrorSeverity.WARNING,
      code: "INVALID_AMOUNT",
      userMessage: reason,
      technicalMessage: `Invalid amount: ${reason}`,
      recoverable: true,
      retryable: false,
      suggestion: "Please enter a valid amount",
    });
  }
}

export class InvalidAddressError extends BridgeError {
  constructor(address: string) {
    super({
      category: ErrorCategory.VALIDATION,
      severity: ErrorSeverity.WARNING,
      code: "INVALID_ADDRESS",
      userMessage: "Invalid wallet address",
      technicalMessage: `Invalid address format: ${address}`,
      recoverable: true,
      retryable: false,
      suggestion: "Please enter a valid Ethereum address (0x...)",
    });
  }
}

export class UnsupportedTokenError extends BridgeError {
  constructor(token: string, chain: string) {
    super({
      category: ErrorCategory.VALIDATION,
      severity: ErrorSeverity.ERROR,
      code: "UNSUPPORTED_TOKEN",
      userMessage: `${token} is not supported on ${chain}`,
      technicalMessage: `Token ${token} not supported on ${chain}`,
      recoverable: true,
      retryable: false,
      suggestion: "Please select a different token or chain",
    });
  }
}

/**
 * Parse error from unknown source into BridgeError
 */
export function parseBridgeError(error: unknown): BridgeError {
  // Already a BridgeError
  if (error instanceof BridgeError) {
    return error;
  }

  // Standard Error
  if (error instanceof Error) {
    const message = error.message.toLowerCase();

    // Wallet errors
    if (message.includes("user rejected") || message.includes("user denied")) {
      return new WalletSignatureRejectedError(error);
    }
    if (message.includes("insufficient funds") || message.includes("insufficient balance")) {
      return new InsufficientBalanceError("unknown", "unknown");
    }

    // Network errors
    if (message.includes("timeout") || message.includes("timed out")) {
      return new NetworkTimeoutError(error);
    }
    if (message.includes("network") || message.includes("connection")) {
      return new RPCError(error);
    }

    // Contract errors
    if (message.includes("gas") && message.includes("estimate")) {
      return new GasEstimationError(error);
    }
    if (message.includes("execution reverted") || message.includes("transaction failed")) {
      return new ContractExecutionError(error.message, error);
    }

    // Encryption errors
    if (message.includes("encrypt") || message.includes("ecies")) {
      return new EncryptionError(error);
    }

    // Generic error
    return new BridgeError({
      category: ErrorCategory.UNKNOWN,
      severity: ErrorSeverity.ERROR,
      code: "UNKNOWN_ERROR",
      userMessage: "An unexpected error occurred",
      technicalMessage: error.message,
      recoverable: true,
      retryable: true,
      suggestion: "Please try again. If the problem persists, contact support",
      originalError: error,
    });
  }

  // Unknown error type
  return new BridgeError({
    category: ErrorCategory.UNKNOWN,
    severity: ErrorSeverity.ERROR,
    code: "UNKNOWN_ERROR",
    userMessage: "An unexpected error occurred",
    technicalMessage: String(error),
    recoverable: true,
    retryable: true,
  });
}

/**
 * Format error for display to user
 */
export function formatErrorMessage(error: BridgeError): {
  title: string;
  message: string;
  suggestion?: string;
  canRetry: boolean;
} {
  return {
    title: `${error.category} Error`,
    message: error.userMessage,
    suggestion: error.suggestion,
    canRetry: error.retryable,
  };
}

/**
 * Log error for debugging/monitoring
 */
export function logBridgeError(error: BridgeError, context?: Record<string, unknown>): void {
  console.error(`[${error.category}][${error.code}] ${error.technicalMessage}`, {
    severity: error.severity,
    recoverable: error.recoverable,
    retryable: error.retryable,
    suggestion: error.suggestion,
    originalError: error.originalError,
    context,
  });

  // In production, send to error tracking service (e.g., Sentry)
  if (typeof window !== "undefined" && (window as any).Sentry) {
    (window as any).Sentry.captureException(error, {
      level: error.severity.toLowerCase(),
      contexts: {
        bridge: context,
      },
    });
  }
}
