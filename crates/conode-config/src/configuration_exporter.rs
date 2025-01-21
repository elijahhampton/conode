use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcConfig {
    pub rpc: String,
    pub chain_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountConfig {
    pub mnemonic: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractConfig {
    /// Payment token smart contract address
    pub payment_token: String,
    /// CoNode smart contract address
    pub conode: String,
    /// The block number containing the DeployTransaction for
    /// conode smart contracts
    pub conode_deployment_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCriteria {
    pub min_reward: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeConfig {
    pub data_dir: String,
    pub log_level: String,
    pub rpc: RpcConfig,
    pub account: AccountConfig,
    pub contract: ContractConfig,
    pub task: TaskCriteria,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigurationExporter {
    pub config: NodeConfig,
}

impl TryFrom<Config> for ConfigurationExporter {
    type Error = ConfigError;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let data_dir = config.get_string("data_dir")?;
        let log_level = config.get_string("log_level")?;

        let rpc = RpcConfig {
            rpc: config.get_string("rpc.rpc")?,
            chain_id: config.get_string("rpc.chain_id")?,
        };

        let account = AccountConfig {
            mnemonic: config.get_string("account.mnemonic")?,
        };

        let contract = ContractConfig {
            payment_token: config.get_string("contract.payment_token")?,
            conode: config.get_string("contract.conode")?,
            conode_deployment_block: config
                .get_int("contract.conode_deployment_block")?
                .try_into()
                .expect("missing config variable `contract.conode_deployment_block`"),
        };

        let task = TaskCriteria {
            min_reward: config
                .get_int("task_criteria.min_reward")?
                .try_into()
                .unwrap_or(0),
        };

        Ok(ConfigurationExporter {
            config: NodeConfig {
                data_dir,
                log_level,
                rpc,
                account,
                contract,
                task,
            },
        })
    }
}

impl Default for ConfigurationExporter {
    fn default() -> Self {
        ConfigurationExporter {
            config: NodeConfig {
                data_dir: "./data".to_string(),
                log_level: "info".to_string(),
                rpc: RpcConfig {
                    rpc: "http://127.0.0.1:5050".to_string(),
                    chain_id: "".to_string(),
                },
                account: AccountConfig {
                    mnemonic: "".to_string(),
                },
                contract: ContractConfig {
                    payment_token: "".to_string(),
                    conode: "".to_string(),
                    conode_deployment_block: 0,
                },
                task: TaskCriteria { min_reward: 0 },
            },
        }
    }
}

impl ConfigurationExporter {
    pub fn new() -> Result<Self, ConfigError> {
        let config_path = env::var("CONODE_CONFIG")
            .map_err(|_| {
                ConfigError::Message("CONODE_CONFIG environment variable must be set".to_string())
            })
            .unwrap();

        Config::builder()
            .add_source(File::with_name(&config_path).required(true))
            .build()?
            .try_into()
    }

    /// Loads the configuration from a TOML file, overriding current settings.
    pub fn load(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let loaded_config: Self = toml::from_str(&contents)?;
        *self = loaded_config;
        Ok(())
    }
}
