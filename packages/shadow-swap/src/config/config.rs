use anyhow::{Result, anyhow};
use std::{env, path::PathBuf};

use crate::{
    models::model::{BridgeConfig, DatabaseConfig, ServerConfig},
    relay_coordinator::model::{EthereumConfig, MantleConfig},
};

impl BridgeConfig {
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file: {}", e))?;

        let config: Self =
            toml::from_str(&contents).map_err(|e| anyhow!("Failed to parse config: {}", e))?;

        Ok(config)
    }

    pub fn from_env() -> Result<Self> {
        Ok(BridgeConfig {
            server: ServerConfig {
                host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|e| anyhow!("Invalid PORT: {}", e))?,
                hmac_secret: env::var("HMAC_SECRET")
                    .map_err(|_| anyhow!("HMAC_SECRET must be set"))?,
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL").map_err(|_| anyhow!("DATABASE_URL must be set"))?,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .map_err(|e| anyhow!("Invalid DB_MAX_CONNECTIONS: {}", e))?,
            },
            ethereum: EthereumConfig::from_env()?,
            mantle: MantleConfig::from_env()?,
            relayer_address: env::var("RELAYER_ADDRESS")
                .map_err(|_| anyhow!("RELAYER_ADDRESS must be set"))?,
            fee_collector: env::var("FEE_COLLECTOR")
                .map_err(|_| anyhow!("FEE_COLLECTOR must be set"))?,
        })
    }
}

impl EthereumConfig {
    pub fn from_env() -> Result<Self> {
        Ok(EthereumConfig {
            rpc_url: env::var("ETHEREUM_RPC_URL")
                .map_err(|_| anyhow!("ETHEREUM_RPC_URL must be set"))?,
            ws_url: env::var("ETHEREUM_WS_URL").ok(),
            private_key: env::var("ETHEREUM_PRIVATE_KEY")
                .map_err(|_| anyhow!("ETHEREUM_PRIVATE_KEY must be set"))?,
            intent_pool_address: env::var("ETHEREUM_INTENT_POOL_ADDRESS")
                .map_err(|_| anyhow!("ETHEREUM_INTENT_POOL_ADDRESS must be set"))?,
            settlement_address: env::var("ETHEREUM_SETTLEMENT_ADDRESS")
                .map_err(|_| anyhow!("ETHEREUM_SETTLEMENT_ADDRESS must be set"))?,
            chain_id: env::var("ETHEREUM_CHAIN_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|e| anyhow!("Invalid ETHEREUM_CHAIN_ID: {}", e))?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if !self.rpc_url.starts_with("http") {
            return Err(anyhow!("Invalid RPC URL format"));
        }

        if self.private_key.len() != 64 && self.private_key.len() != 66 {
            return Err(anyhow!("Invalid private key length"));
        }

        if !self.intent_pool_address.starts_with("0x") || self.intent_pool_address.len() != 42 {
            return Err(anyhow!("Invalid intent pool address"));
        }

        if !self.settlement_address.starts_with("0x") || self.settlement_address.len() != 42 {
            return Err(anyhow!("Invalid settlement address"));
        }

        Ok(())
    }
}

impl MantleConfig {
    pub fn from_env() -> Result<Self> {
        Ok(MantleConfig {
            rpc_url: env::var("MANTLE_RPC_URL")
                .map_err(|_| anyhow!("MANTLE_RPC_URL must be set"))?,
            ws_url: env::var("MANTLE_WS_URL").ok(),
            private_key: env::var("MANTLE_PRIVATE_KEY")
                .map_err(|_| anyhow!("MANTLE_PRIVATE_KEY must be set"))?,
            intent_pool_address: env::var("MANTLE_INTENT_POOL_ADDRESS")
                .map_err(|_| anyhow!("MANTLE_INTENT_POOL_ADDRESS must be set"))?,
            settlement_address: env::var("MANTLE_SETTLEMENT_ADDRESS")
                .map_err(|_| anyhow!("MANTLE_SETTLEMENT_ADDRESS must be set"))?,
            chain_id: env::var("MANTLE_CHAIN_ID")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .map_err(|e| anyhow!("Invalid MANTLE_CHAIN_ID: {}", e))?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if !self.rpc_url.starts_with("http") {
            return Err(anyhow!("Invalid RPC URL format"));
        }

        if self.private_key.len() != 64 && self.private_key.len() != 66 {
            return Err(anyhow!("Invalid private key length"));
        }

        if !self.intent_pool_address.starts_with("0x") || self.intent_pool_address.len() != 42 {
            return Err(anyhow!("Invalid intent pool address"));
        }

        if !self.settlement_address.starts_with("0x") || self.settlement_address.len() != 42 {
            return Err(anyhow!("Invalid settlement address"));
        }

        Ok(())
    }
}

