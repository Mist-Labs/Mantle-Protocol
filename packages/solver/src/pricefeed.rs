use anyhow::{Result, anyhow};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tracing::{info, warn};

use crate::model::SupportedToken;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceData {
    pub price: f64,
    pub timestamp: i64,
    pub sources: Vec<SourcePrice>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourcePrice {
    pub source: String,
    pub price: f64,
}

pub struct PriceFeedManager {
    cache: Arc<RwLock<HashMap<String, PriceData>>>,
    client: Client,
}

impl PriceFeedManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
        }
    }

    pub async fn init(&self) {
        info!("🔄 Initializing price feeds");

        let symbols = vec!["ETH", "WETH", "MNT"];

        for symbol in symbols {
            self.update_price_for_pair(symbol, "USD").await;
        }

        self.start_background_updates().await;
    }

    async fn start_background_updates(&self) {
        let cache_clone = self.cache.clone();
        let client_clone = self.client.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(120));

            loop {
                interval.tick().await;

                let pairs = vec![("ETH", "USD"), ("WETH", "USD"), ("MNT", "USD")];

                for (from, to) in pairs {
                    if let Err(e) =
                        Self::fetch_and_update_price(&client_clone, &cache_clone, from, to).await
                    {
                        warn!("Failed to update {}-{}: {}", from, to, e);
                    }
                }
            }
        });

        info!("✅ Background price updates started (60s interval)");
    }

    async fn update_price_for_pair(&self, from_symbol: &str, to_symbol: &str) {
        if let Err(e) =
            Self::fetch_and_update_price(&self.client, &self.cache, from_symbol, to_symbol).await
        {
            warn!(
                "Failed to fetch initial price for {}-{}: {}",
                from_symbol, to_symbol, e
            );
        }
    }

    async fn fetch_and_update_price(
        client: &Client,
        cache: &Arc<RwLock<HashMap<String, PriceData>>>,
        from_symbol: &str,
        to_symbol: &str,
    ) -> Result<()> {
        let mut sources = Vec::new();
        let mut sum = 0.0;
        let mut count = 0;

        if from_symbol != "MNT" {
            if let Ok(price) = Self::get_cryptocompare_price(client, from_symbol, to_symbol).await {
                sources.push(SourcePrice {
                    source: "CryptoCompare".to_string(),
                    price,
                });
                sum += price;
                count += 1;
            }
        }

        if let Ok(price) = Self::get_coingecko_price(client, from_symbol, to_symbol).await {
            sources.push(SourcePrice {
                source: "CoinGecko".to_string(),
                price,
            });
            sum += price;
            count += 1;
        }

        if let Ok(price) = Self::get_gateio_price(client, from_symbol).await {
            sources.push(SourcePrice {
                source: "Gate.io".to_string(),
                price,
            });
            sum += price;
            count += 1;
        }

        if let Ok(price) = Self::get_mexc_price(client, from_symbol).await {
            sources.push(SourcePrice {
                source: "MEXC".to_string(),
                price,
            });
            sum += price;
            count += 1;
        }

        if count > 0 {
            let average_price = sum / count as f64;
            let pair_key = format!("{}-{}", from_symbol, to_symbol);

            let price_data = PriceData {
                price: average_price,
                timestamp: Utc::now().timestamp(),
                sources: sources.clone(),
            };

            let mut cache_guard = cache.write().await;
            cache_guard.insert(pair_key.clone(), price_data);

            let source_names: Vec<String> = sources.iter().map(|s| s.source.clone()).collect();
            info!(
                "💰 Price updated: {} = ${:.4} (from {} sources: {})",
                pair_key,
                average_price,
                count,
                source_names.join(", ")
            );
            Ok(())
        } else {
            Err(anyhow!("Failed to fetch price from all sources"))
        }
    }

    pub async fn get_usd_price(&self, token: SupportedToken) -> Result<f64> {
        let symbol = token.symbol();

        if symbol == "USDC" || symbol == "USDT" {
            return Ok(1.0);
        }

        let pair_key = format!("{}-USD", symbol);
        let cache = self.cache.read().await;

        if let Some(price_data) = cache.get(&pair_key) {
            let age = Utc::now().timestamp() - price_data.timestamp;

            if age > 120 {
                warn!(
                    "⚠️ Price data for {} is stale ({} seconds old)",
                    pair_key, age
                );
            }

            if price_data.price > 0.0 {
                return Ok(price_data.price);
            }
        }

        Err(anyhow!("No valid price data for {}", symbol))
    }

    async fn get_cryptocompare_price(
        client: &Client,
        from_symbol: &str,
        to_symbol: &str,
    ) -> Result<f64> {
        let url = format!(
            "https://min-api.cryptocompare.com/data/price?fsym={}&tsyms={}",
            from_symbol, to_symbol
        );
        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            let price = data[to_symbol]
                .as_f64()
                .ok_or_else(|| anyhow!("Invalid price format"))?;
            Ok(price)
        } else {
            Err(anyhow!("API error: {}", response.status()))
        }
    }

    async fn get_coingecko_price(
        client: &Client,
        from_symbol: &str,
        to_symbol: &str,
    ) -> Result<f64> {
        let from_id = match from_symbol.to_uppercase().as_str() {
            "ETH" | "WETH" => "ethereum",
            "USDC" => "usd-coin",
            "USDT" => "tether",
            "MNT" => "mantle",
            _ => return Err(anyhow!("Unsupported symbol: {}", from_symbol)),
        };

        let to_currency = to_symbol.to_lowercase();
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}",
            from_id, to_currency
        );

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            let price = data[from_id][&to_currency]
                .as_f64()
                .ok_or_else(|| anyhow!("Invalid price format"))?;
            Ok(price)
        } else {
            Err(anyhow!("API error: {}", response.status()))
        }
    }

    async fn get_gateio_price(client: &Client, from_symbol: &str) -> Result<f64> {
        let pair = format!("{}_USDT", from_symbol.to_uppercase());
        let url = format!(
            "https://api.gateio.ws/api/v4/spot/tickers?currency_pair={}",
            pair
        );

        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            if let Some(ticker) = data.as_array().and_then(|arr| arr.first()) {
                let price = ticker["last"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .ok_or_else(|| anyhow!("Invalid price format"))?;
                Ok(price)
            } else {
                Err(anyhow!("No ticker data"))
            }
        } else {
            Err(anyhow!("API error: {}", response.status()))
        }
    }

    async fn get_mexc_price(client: &Client, from_symbol: &str) -> Result<f64> {
        let symbol = format!("{}USDT", from_symbol.to_uppercase());
        let url = format!("https://api.mexc.com/api/v3/ticker/price?symbol={}", symbol);

        let response = client.get(&url).send().await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            let price = data["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| anyhow!("Invalid price format"))?;
            Ok(price)
        } else {
            Err(anyhow!("API error: {}", response.status()))
        }
    }
}
