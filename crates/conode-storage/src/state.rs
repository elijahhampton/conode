use super::manager::chain::ChainManager;
use crate::manager::{storage::StorageManager, sync::SyncManager};
use anyhow;
use conode_config::configuration_exporter::ConfigurationExporter;
use conode_types::chain::ChainConfig;
use libp2p::PeerId;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use starknet::{core::types::Felt, signers::LocalWallet};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A shared state between the CoNodeNetwork and arbitrary frontend(s) that need access to
/// the underlying resources, including, a ConfigurationExporter, ChainManager, SyncManager, StorageManager
/// and the local peer id of the node.
pub struct SharedState {
    pub local_peer_id: PeerId,
    #[allow(dead_code)]
    configuration_exporter: ConfigurationExporter,
    pub chain_manager: ChainManager,
    pub sync_manager: SyncManager,
    pub storage_manager: StorageManager,
}

// A thread safe and interior mutable reference to the SharedState.
pub type SharedStateArc = Arc<Mutex<SharedState>>;

// Creates the shared state from a local peer id and a starknet::LocalWallet.
pub fn create_shared_state(
    wallet: LocalWallet,
    local_peer_id: PeerId,
    starknet_address: Felt,
) -> anyhow::Result<SharedStateArc> {
    // Configuration values
    let configuration_exporter = ConfigurationExporter::new()?;
    let rpc = configuration_exporter.config.rpc.rpc.clone();
    let chain_id = configuration_exporter.config.rpc.chain_id.clone();
    let conode_contract_address = configuration_exporter.config.contract.conode.clone();
    let payment_token_contract_address =
        configuration_exporter.config.contract.payment_token.clone();
    let data_dir = configuration_exporter.config.data_dir.clone();

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let cf_names = vec![
        "jobs",
        "user_jobs",
        "proposals",
        "gossiped_work",
        "active_jobs",
        "active_work",
        "broadcasted_work",
        "synced_block_number",
        "work_submissions",
        "work_salt",
        "solutions",
        "solution_salts",
        "solution_hashes",
    ];

    let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
        .iter()
        .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
        .collect();

    // We only wrap the DB in Arc since it's specifically designed for shared access
    let db = Arc::new(DB::open_cf_descriptors(&opts, data_dir, cf_descriptors)?);

    let chain_config = ChainConfig {
        rpc,
        chain_id,
        conode_contract: conode_contract_address,
        payment_token_contract: payment_token_contract_address,
    };

    let chain_manager = ChainManager::new(wallet, starknet_address, chain_config.clone()).unwrap();
    let storage_manager = StorageManager::new(Arc::clone(&db))?;
    let sync_manager =
        SyncManager::new(storage_manager.clone(), chain_manager.clone(), chain_config);

    let shared_state = SharedState {
        local_peer_id,
        chain_manager,
        configuration_exporter,
        sync_manager,
        storage_manager,
    };

    // Single Arc<Mutex> wrapper for the entire state
    Ok(Arc::new(Mutex::new(shared_state)))
}
