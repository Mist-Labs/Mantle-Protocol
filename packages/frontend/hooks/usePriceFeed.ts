/**
 * usePriceFeed Hook
 *
 * Hook for fetching and managing token price data from the backend API
 */

import { useState, useEffect, useCallback } from "react";
import { getAllPrices, getExchangeRate, convertAmount } from "@/lib/api";

/**
 * Price feed state
 */
interface PriceFeedState {
  prices: Record<string, number>;
  isLoading: boolean;
  error: string | null;
  lastUpdated: number | null;
}

/**
 * Hook for fetching all token prices
 */
export function useAllPrices(refreshInterval: number = 60000) {
  const [state, setState] = useState<PriceFeedState>({
    prices: {},
    isLoading: true,
    error: null,
    lastUpdated: null,
  });

  const fetchPrices = useCallback(async () => {
    try {
      setState((prev) => ({ ...prev, isLoading: true, error: null }));

      const response = await getAllPrices();

      setState({
        prices: response.prices,
        isLoading: false,
        error: null,
        lastUpdated: response.timestamp,
      });
    } catch (error) {
      console.error("Failed to fetch prices:", error);
      setState((prev) => ({
        ...prev,
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to fetch prices",
      }));
    }
  }, []);

  // Initial fetch
  useEffect(() => {
    fetchPrices();
  }, [fetchPrices]);

  // Auto-refresh
  useEffect(() => {
    if (refreshInterval > 0) {
      const interval = setInterval(fetchPrices, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [refreshInterval, fetchPrices]);

  return {
    ...state,
    refetch: fetchPrices,
  };
}

/**
 * Hook for getting USD value of a token amount
 */
export function useTokenUSDValue(tokenSymbol: string, amount: string) {
  const { prices, isLoading } = useAllPrices();
  const [usdValue, setUsdValue] = useState<number | null>(null);

  useEffect(() => {
    if (!amount || isNaN(Number(amount)) || Number(amount) <= 0) {
      setUsdValue(null);
      return;
    }

    const priceKey = `${tokenSymbol.toUpperCase()}-USD`;
    const price = prices[priceKey];

    if (price) {
      setUsdValue(Number(amount) * price);
    } else {
      setUsdValue(null);
    }
  }, [tokenSymbol, amount, prices]);

  return {
    usdValue,
    isLoading,
    price: prices[`${tokenSymbol.toUpperCase()}-USD`] || null,
  };
}

/**
 * Hook for token-to-token conversion
 */
export function useTokenConversion(
  fromSymbol: string,
  toSymbol: string,
  amount: string
) {
  const [state, setState] = useState<{
    outputAmount: number | null;
    rate: number | null;
    isLoading: boolean;
    error: string | null;
  }>({
    outputAmount: null,
    rate: null,
    isLoading: false,
    error: null,
  });

  useEffect(() => {
    if (!amount || isNaN(Number(amount)) || Number(amount) <= 0) {
      setState({
        outputAmount: null,
        rate: null,
        isLoading: false,
        error: null,
      });
      return;
    }

    const fetchConversion = async () => {
      try {
        setState((prev) => ({ ...prev, isLoading: true, error: null }));

        const response = await convertAmount(
          fromSymbol,
          toSymbol,
          Number(amount)
        );

        setState({
          outputAmount: response.output_amount,
          rate: response.rate,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        console.error("Failed to convert amount:", error);
        setState({
          outputAmount: null,
          rate: null,
          isLoading: false,
          error:
            error instanceof Error ? error.message : "Failed to convert amount",
        });
      }
    };

    // Debounce the API call
    const timeoutId = setTimeout(fetchConversion, 500);

    return () => clearTimeout(timeoutId);
  }, [fromSymbol, toSymbol, amount]);

  return state;
}

/**
 * Hook for getting exchange rate between two tokens
 */
export function useExchangeRate(fromSymbol: string, toSymbol: string) {
  const [state, setState] = useState<{
    rate: number | null;
    isLoading: boolean;
    error: string | null;
  }>({
    rate: null,
    isLoading: true,
    error: null,
  });

  useEffect(() => {
    const fetchRate = async () => {
      try {
        setState((prev) => ({ ...prev, isLoading: true, error: null }));

        const response = await getExchangeRate(fromSymbol, toSymbol);

        setState({
          rate: response.rate,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        console.error("Failed to fetch exchange rate:", error);
        setState({
          rate: null,
          isLoading: false,
          error:
            error instanceof Error
              ? error.message
              : "Failed to fetch exchange rate",
        });
      }
    };

    fetchRate();

    // Refresh every minute
    const interval = setInterval(fetchRate, 60000);
    return () => clearInterval(interval);
  }, [fromSymbol, toSymbol]);

  return state;
}
