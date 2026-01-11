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
  generatePrivacyParams,
  getEIP712Domain,
  CLAIM_AUTH_TYPES,
  type ClaimAuthMessage,
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
        if (!address || !walletClient) {
          throw new Error("Wallet not connected");
        }

        updateState({
          isLoading: true,
          step: "generating-params",
          error: null,
        });

        // Validate chain matches source chain
        const sourceChainId = getChainId(params.sourceChain);
        if (chain?.id !== sourceChainId) {
          throw new Error(
            `Please switch to ${params.sourceChain} network`
          );
        }

        // Get token info
        const tokenInfo = getTokenInfo(params.tokenSymbol, params.sourceChain);
        if (!tokenInfo) {
          throw new Error(`Token ${params.tokenSymbol} not supported on ${params.sourceChain}`);
        }

        // Parse amount to wei
        const amountWei = parseTokenAmount(params.amount, tokenInfo.decimals);

        // Step 1: Generate privacy parameters
        const privacyParams = generatePrivacyParams(
          address,
          tokenInfo.address,
          amountWei
        );

        console.log("Privacy params generated:", {
          intentId: privacyParams.intentId,
          commitment: privacyParams.commitment,
        });

        // Step 2: Sign claim authorization (EIP-712)
        updateState({ step: "signing-auth" });

        const destChainId = getChainId(params.destChain);
        const destContracts = getContractsForChain(params.destChain);

        if (!destContracts.settlement) {
          throw new Error(`Settlement contract not configured for ${params.destChain}`);
        }

        // Use SOURCE chain ID for EIP-712 domain since user is connected to source chain
        // The signature will still be valid for claim authorization on destination chain
        const domain = getEIP712Domain(sourceChainId, destContracts.settlement);

        const message: ClaimAuthMessage = {
          intentId: privacyParams.intentId,
          recipient: params.recipient,
          token: tokenInfo.address,
          amount: amountWei,
        };

        const claimAuth = await walletClient.signTypedData({
          account: address,
          domain,
          types: {
            ClaimAuthorization: CLAIM_AUTH_TYPES,
          },
          primaryType: "ClaimAuthorization",
          message,
        });

        console.log("Claim authorization signed");

        // Step 3: Approve token if ERC20
        if (!isNativeToken(tokenInfo.address)) {
          updateState({ step: "approving-token" });

          const sourceContracts = getContractsForChain(params.sourceChain);

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
        }

        // Step 4: Create intent on-chain
        updateState({ step: "creating-intent" });

        const sourceContracts = getContractsForChain(params.sourceChain);

        const txHash = await writeContractAsync({
          address: sourceContracts.intentPool,
          abi: INTENT_POOL_ABI,
          functionName: "createIntent",
          args: [
            privacyParams.intentId,
            privacyParams.commitment,
            tokenInfo.address,
            amountWei,
            destChainId,
            address, // refundTo
            BigInt(0), // customDeadline (0 = use default 1 hour)
          ],
          value: isNativeToken(tokenInfo.address) ? amountWei : BigInt(0),
        });

        console.log("Intent created on-chain:", txHash);

        updateState({ txHash });

        // Wait for transaction confirmation
        await publicClient!.waitForTransactionReceipt({ hash: txHash });

        // Step 5: Encrypt privacy parameters and submit to backend
        updateState({ step: "submitting-backend" });

        // Get destination token address
        const destTokenInfo = getTokenInfo(params.tokenSymbol, params.destChain);
        if (!destTokenInfo) {
          throw new Error(`Token ${params.tokenSymbol} not supported on ${params.destChain}`);
        }

        // CRITICAL: Encrypt secret and nullifier before sending to backend
        // This ensures privacy parameters are never transmitted in plain text
        const { encryptedSecret, encryptedNullifier } = await encryptPrivacyParams(
          privacyParams.secret,
          privacyParams.nullifier
        );

        console.log("Privacy parameters encrypted for transmission");

        const backendResponse = await initiateBridge({
          user_address: address,
          source_chain: params.sourceChain,
          dest_chain: params.destChain,
          source_token: tokenInfo.address,
          dest_token: destTokenInfo.address,
          amount: amountWei.toString(),
          commitment: privacyParams.commitment,
          secret: encryptedSecret, // Send encrypted secret
          nullifier: encryptedNullifier, // Send encrypted nullifier
          claim_auth: claimAuth,
          recipient: params.recipient,
          refund_address: address,
        });

        console.log("Backend notified:", backendResponse);

        updateState({
          intentId: backendResponse.intent_id as Hex,
          step: "waiting-solver",
        });

        // Step 6: Poll for status updates
        const finalStatus = await pollIntentStatus(
          backendResponse.intent_id,
          (statusUpdate: IntentStatusResponse) => {
            updateState({ status: statusUpdate.status });
            console.log("Status update:", statusUpdate.status);
          }
        );

        updateState({
          step: finalStatus.status === "completed" ? "completed" : "failed",
          status: finalStatus.status,
          isLoading: false,
        });

        return finalStatus;
      } catch (error: unknown) {
        console.error("Bridge error:", error);

        const errorMessage =
          error instanceof Error ? error.message : "Bridge transaction failed";

        updateState({
          step: "failed",
          error: errorMessage,
          isLoading: false,
        });

        throw error;
      }
    },
    [address, walletClient, chain, publicClient, writeContractAsync, updateState]
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
