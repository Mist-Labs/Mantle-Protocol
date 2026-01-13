/**
 * useBridge Hook
 *
 * Main hook for orchestrating the complete privacy bridge flow:
 * 1. Generate privacy parameters (client-side)
 * 2. Sign claim authorization (EIP-712)
 * 3. Approve token (if ERC20)
 * 4. Create intent on-chain
 * 5. Submit to backend
 * 6. Poll for status updates
 */

import { useState, useCallback } from "react";
import {
  useAccount,
  useWalletClient,
  usePublicClient,
  useWriteContract,
  useWaitForTransactionReceipt,
} from "wagmi";
import type { Hex } from "viem";
import { parseUnits, type Address } from "viem";

import {
  generateSecret,
  generateNullifier,
  generateIntentId,
  computePoseidonCommitment,
  generateClaimAuthHash,
} from "@/lib/privacy";
import {
  initiateBridge,
  pollIntentStatus,
  type IntentStatus,
  type IntentStatusResponse,
} from "@/lib/api";
import {
  INTENT_POOL_ABI,
  ERC20_ABI,
  POSEIDON_ABI,
  getContractsForChain,
  getChainId,
} from "@/lib/contracts";
import {
  getTokenInfo,
  isNativeToken,
  parseTokenAmount,
  type ChainType,
} from "@/lib/tokens";
import { encryptPrivacyParams } from "@/lib/encryption";
import {
  parseBridgeError,
  logBridgeError,
  WalletNotConnectedError,
  WrongNetworkError,
  UnsupportedTokenError,
  WalletSignatureRejectedError,
  TokenApprovalError,
  ContractExecutionError,
  EncryptionError,
  PoseidonHashError,
} from "@/lib/errors";
import { useTransactionHistory } from "./useTransactionHistory";

/**
 * Bridge parameters
 */
export interface BridgeParams {
  sourceChain: ChainType;
  destChain: ChainType;
  tokenSymbol: string;
  amount: string; // Human-readable amount (e.g., "1.5")
  recipient: Address;
}

/**
 * Bridge step status
 */
export type BridgeStep =
  | "idle"
  | "generating-params"
  | "signing-auth"
  | "approving-token"
  | "creating-intent"
  | "submitting-backend"
  | "waiting-solver"
  | "completed"
  | "failed";

/**
 * Bridge state
 */
export interface BridgeState {
  step: BridgeStep;
  intentId: Hex | null;
  txHash: Hex | null;
  status: IntentStatus | null;
  error: string | null;
  isLoading: boolean;
}

/**
 * Main bridge hook
 */
export function useBridge() {
  const { address, chain } = useAccount();
  const { data: walletClient } = useWalletClient();
  const publicClient = usePublicClient();
  const { writeContractAsync } = useWriteContract();
  const { addTransaction, updateTransaction } = useTransactionHistory(address);

  const [state, setState] = useState<BridgeState>({
    step: "idle",
    intentId: null,
    txHash: null,
    status: null,
    error: null,
    isLoading: false,
  });

  /**
   * Update bridge state
   */
  const updateState = useCallback((updates: Partial<BridgeState>) => {
    setState((prev) => ({ ...prev, ...updates }));
  }, []);

  /**
   * Reset bridge state
   */
  const reset = useCallback(() => {
    setState({
      step: "idle",
      intentId: null,
      txHash: null,
      status: null,
      error: null,
      isLoading: false,
    });
  }, []);

  /**
   * Execute bridge transaction
   */
  const bridge = useCallback(
    async (params: BridgeParams) => {
      try {
        // Validate wallet connection
        if (!address || !walletClient) {
          throw new WalletNotConnectedError();
        }

        updateState({
          isLoading: true,
          step: "generating-params",
          error: null,
        });

        // Validate chain matches source chain
        const sourceChainId = getChainId(params.sourceChain);
        if (chain?.id !== sourceChainId) {
          throw new WrongNetworkError(
            params.sourceChain,
            chain?.name || "unknown"
          );
        }

        // Get token info and validate support
        const tokenInfo = getTokenInfo(params.tokenSymbol, params.sourceChain);
        if (!tokenInfo) {
          throw new UnsupportedTokenError(
            params.tokenSymbol,
            params.sourceChain
          );
        }

        // Parse amount to wei
        const amountWei = parseTokenAmount(params.amount, tokenInfo.decimals);

        // Step 1: Generate privacy parameters
        let privacyParams;
        try {
          // Generate random secret
          const secret = generateSecret();
          const nullifier = generateNullifier(secret);
          const intentId = generateIntentId(address, tokenInfo.address, amountWei);

          // Compute commitment using Poseidon contract
          const sourceContracts = getContractsForChain(params.sourceChain);

          // Create a helper function to call Poseidon contract
          const callPoseidon = async (inputs: readonly Hex[]) => {
            const result = await publicClient!.readContract({
              address: sourceContracts.poseidonHasher,
              abi: POSEIDON_ABI,
              functionName: "poseidon",
              args: [inputs as readonly [Hex, Hex, Hex, Hex]],
            });
            return result;
          };

          const commitment = await computePoseidonCommitment(
            secret,
            nullifier,
            amountWei,
            sourceChainId,
            callPoseidon
          );

          // CRITICAL: Validate commitment is not zeros
          if (!commitment || commitment === "0x0000000000000000000000000000000000000000000000000000000000000000") {
            throw new Error(
              "Poseidon hash returned invalid commitment (zeros). " +
              "This indicates a problem with the Poseidon contract call. " +
              "Please check your RPC connection and contract address."
            );
          }

          privacyParams = {
            intentId,
            secret,
            nullifier,
            commitment,
          };

          console.log("Privacy params generated:", {
            intentId: privacyParams.intentId,
            secret: secret,
            nullifier: nullifier,
            commitment: privacyParams.commitment,
            sourceChainId: sourceChainId,
            amount: amountWei.toString(),
          });
        } catch (error) {
          throw new PoseidonHashError(error instanceof Error ? error : undefined);
        }

        // Step 2: Sign claim authorization
        // Generate authHash = keccak256(abi.encodePacked(intentId, nullifier, recipient))
        // Relayer will use this signature to claim on behalf of the user
        updateState({ step: "signing-auth" });

        let claimAuth: Hex;
        try {
          // Generate the auth hash that matches contract's verification
          // Contract does: keccak256(abi.encodePacked(intentId, nullifier, recipient))
          const authHash = generateClaimAuthHash(
            privacyParams.intentId,
            privacyParams.nullifier,
            params.recipient
          );

          // Sign the auth hash using personal_sign
          // This automatically adds the "\x19Ethereum Signed Message:\n32" prefix
          claimAuth = await walletClient.signMessage({
            account: address,
            message: { raw: authHash },
          });

          console.log("Claim authorization signed");
        } catch (error) {
          throw new WalletSignatureRejectedError(
            error instanceof Error ? error : undefined
          );
        }

        // Step 3: Approve token if ERC20
        if (!isNativeToken(tokenInfo.address)) {
          updateState({ step: "approving-token" });

          const sourceContracts = getContractsForChain(params.sourceChain);

          try {
            // Check current allowance
            const allowance = await publicClient!.readContract({
              address: tokenInfo.address,
              abi: ERC20_ABI,
              functionName: "allowance",
              args: [address, sourceContracts.intentPool],
            });

            // Approve if needed
            if (allowance < amountWei) {
              const approveTx = await writeContractAsync({
                address: tokenInfo.address,
                abi: ERC20_ABI,
                functionName: "approve",
                args: [sourceContracts.intentPool, amountWei],
              });

              // Wait for approval confirmation
              await publicClient!.waitForTransactionReceipt({
                hash: approveTx,
              });

              console.log("Token approved:", approveTx);
            }
          } catch (error) {
            throw new TokenApprovalError(error instanceof Error ? error : undefined);
          }
        }

        // Step 4: Create intent on-chain
        updateState({ step: "creating-intent" });

        const sourceContracts = getContractsForChain(params.sourceChain);
        const destChainId = getChainId(params.destChain);

        // Get destination token address
        const destTokenInfo = getTokenInfo(params.tokenSymbol, params.destChain);
        if (!destTokenInfo) {
          throw new UnsupportedTokenError(params.tokenSymbol, params.destChain);
        }

        // Validate token is supported on-chain
        try {
          const isSupported = await publicClient!.readContract({
            address: sourceContracts.intentPool,
            abi: INTENT_POOL_ABI,
            functionName: "isTokenSupported",
            args: [tokenInfo.address],
          });

          if (!isSupported) {
            throw new Error(
              `Token ${tokenInfo.symbol} (${tokenInfo.address}) is not configured in the contract. ` +
              `Please contact the team to add this token.`
            );
          }

          // Check if contract is paused
          const isPaused = await publicClient!.readContract({
            address: sourceContracts.intentPool,
            abi: INTENT_POOL_ABI,
            functionName: "paused",
          });

          if (isPaused) {
            throw new Error("Bridge contract is currently paused. Please try again later.");
          }

          console.log(`Token ${tokenInfo.symbol} is supported on-chain`);
        } catch (error) {
          if (error instanceof Error && error.message.includes("not configured")) {
            throw error;
          }
          console.warn("Could not validate token support on-chain:", error);
          // Continue anyway - validation is best-effort
        }

        // Calculate destination amount (same as source for now, fees handled on settlement)
        const destAmountWei = amountWei;

        let txHash: Hex;
        try {
          txHash = await writeContractAsync({
            address: sourceContracts.intentPool,
            abi: INTENT_POOL_ABI,
            functionName: "createIntent",
            args: [
              privacyParams.intentId,
              privacyParams.commitment,
              tokenInfo.address,
              amountWei,
              destTokenInfo.address,
              destAmountWei,
              destChainId,
              address, // refundTo
              BigInt(0), // customDeadline (0 = use default 2 hour timeout)
            ],
            value: isNativeToken(tokenInfo.address) ? amountWei : BigInt(0),
          });

          console.log("Intent created on-chain:", txHash);

          updateState({ txHash });

          // Wait for transaction confirmation
          await publicClient!.waitForTransactionReceipt({ hash: txHash });

          // Save transaction to history
          if (address) {
            addTransaction({
              intentId: privacyParams.intentId,
              txHash,
              userAddress: address,
              sourceChain: params.sourceChain,
              destChain: params.destChain,
              sourceToken: tokenInfo.symbol,
              destToken: params.tokenSymbol,
              amount: params.amount,
              status: "committed",
            });
          }
        } catch (error) {
          throw new ContractExecutionError(
            "Failed to create intent on-chain",
            error instanceof Error ? error : undefined
          );
        }

        // Step 5: Encrypt privacy parameters and submit to backend
        updateState({ step: "submitting-backend" });

        // CRITICAL: Encrypt secret and nullifier before sending to backend
        // This ensures privacy parameters are never transmitted in plain text
        let encryptedSecret: string;
        let encryptedNullifier: string;
        try {
          const encrypted = await encryptPrivacyParams(
            privacyParams.secret,
            privacyParams.nullifier
          );
          encryptedSecret = encrypted.encryptedSecret;
          encryptedNullifier = encrypted.encryptedNullifier;

          console.log("Privacy parameters encrypted for transmission");
        } catch (error) {
          throw new EncryptionError(error instanceof Error ? error : undefined);
        }

        // Try to notify backend, but don't fail if it times out
        // The transaction is already on-chain and will be processed
        let backendIntentId: string = privacyParams.intentId;
        try {
          const backendResponse = await initiateBridge({
            intent_id: privacyParams.intentId,
            user_address: address,
            source_chain: params.sourceChain,
            dest_chain: params.destChain,
            source_token: tokenInfo.address,
            dest_token: destTokenInfo.address,
            amount: amountWei.toString(),
            commitment: privacyParams.commitment,
            encrypted_secret: encryptedSecret, // ECIES encrypted secret
            encrypted_nullifier: encryptedNullifier, // ECIES encrypted nullifier
            claim_auth: claimAuth,
            recipient: params.recipient,
            refund_address: address,
          });

          backendIntentId = backendResponse.intent_id;
          console.log("Backend notified:", backendResponse);
        } catch (backendError) {
          // Backend notification failed, but transaction is still valid
          console.warn("Backend notification failed (non-critical):", backendError);
          console.log("✅ Transaction is on-chain. Solvers will process it.");
        }

        updateState({
          intentId: backendIntentId as Hex,
          step: "waiting-solver",
        });

        // Step 6: Poll for status updates
        // Increased timeout: 180 attempts × 5 seconds = 15 minutes
        // This allows sufficient time for solver to process and settle the intent
        const terminalStates: IntentStatus[] = ["completed", "refunded", "failed"];

        const finalStatus = await pollIntentStatus(
          backendIntentId,
          (statusUpdate: IntentStatusResponse) => {
            console.log("Status update:", statusUpdate.status);

            // Determine the step based on status
            let newStep: BridgeStep = "waiting-solver";
            if (statusUpdate.status === "completed") {
              newStep = "completed";
            } else if (statusUpdate.status === "failed" || statusUpdate.status === "refunded") {
              newStep = "failed";
            }

            // Update both status and step
            updateState({
              status: statusUpdate.status,
              step: newStep,
              isLoading: !terminalStates.includes(statusUpdate.status),
            });

            // Update transaction history with status changes
            updateTransaction(privacyParams.intentId, {
              status: statusUpdate.status,
            });
          },
          180, // maxAttempts: 15 minutes total
          5000 // intervalMs: check every 5 seconds
        );

        // Check if terminal state was reached
        const isTerminal = terminalStates.includes(finalStatus.status);

        updateState({
          step: isTerminal ? (finalStatus.status === "completed" ? "completed" : "failed") : "completed",
          status: finalStatus.status,
          error: null,
          isLoading: false,
        });

        // Final update to transaction history
        updateTransaction(privacyParams.intentId, {
          status: finalStatus.status,
        });

        return finalStatus;
      } catch (error: unknown) {
        // Parse error into BridgeError format
        const bridgeError = parseBridgeError(error);

        // Log error with context
        logBridgeError(bridgeError, {
          step: state.step,
          sourceChain: params.sourceChain,
          destChain: params.destChain,
          tokenSymbol: params.tokenSymbol,
          amount: params.amount,
        });

        // Update state with user-friendly error message
        updateState({
          step: "failed",
          error: bridgeError.userMessage,
          isLoading: false,
        });

        // Re-throw the parsed error
        throw bridgeError;
      }
    },
    [address, walletClient, chain, publicClient, writeContractAsync, updateState, state.step]
  );

  return {
    bridge,
    reset,
    state,
    isLoading: state.isLoading,
    step: state.step,
    intentId: state.intentId,
    txHash: state.txHash,
    status: state.status,
    error: state.error,
  };
}
