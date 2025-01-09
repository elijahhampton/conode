use anyhow::{Context, Result};
use conode_config::bootnode::BOOTSTRAP_NODES;
use conode_config::configuration_exporter::ConfigurationExporter;
use conode_logging::logger::{log_info, log_warning};
use conode_network::behaviour::DistributedBehaviour;
use conode_network::handle::NetworkHandle;
use conode_network::network::Network;
use conode_network::request::{RequestHandler, RetryConfig};
use conode_starknet::crypto::keypair::KeyPair;
use conode_state::state::service::StateService;
use conode_state::sync::state::SyncState;
use conode_storage::disk_db::{DiskDb};
use conode_storage::manager::chain::ChainContext;
use conode_storage::manager::storage::{InMemoryDb};
use conode_storage::traits::{Storage, StorageDefault};
use conode_types::chain::ChainConfig;
use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::sync::SyncEvent;
use conode_types::work::{ActiveWork, FlattenedWork, WorkBroadcast};
use libp2p::kad::RoutingUpdate;
use libp2p::{Multiaddr, PeerId, SwarmBuilder};
use log::{debug, error, info};
use starknet::core::types::{Felt, InvokeTransactionResult};
use starknet::signers::{LocalWallet, SigningKey};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{event, instrument, span, Level};

use super::event::{NetworkEvent, NodeCommand};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

/// A shareable interface and frontend to interact with CoNode.
pub struct Node {
    // The [`PeerId`] identifying this node
    pub local_peer_id: PeerId,
    // The [`NetworkHandle`] to the underlying [`Network`]
    pub network: NetworkHandle,
    // A [`StateService`] for chain context and syncing
    pub state_service: Arc<StateService>,
    // An InMemoryDb
    mem_db: Arc<tokio::sync::Mutex<InMemoryDb>>,
    pub sync_event_receiver: tokio::sync::watch::Receiver<SyncEvent>
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("network", &"NetworkHandle")
            .field("command_receiver", &"mpsc::Receiver<NodeCommand>")
            .field("event_sender", &"mpsc::Sender<NetworkEvent>")
            .field("runtime", &"tokio::runtime::Runtime")
            .finish()
    }
}

impl Node {
    /// Create a new node
    pub async fn new() -> Result<(Self, PeerId, PeerInfo)> {
        let local_keypair = KeyPair::generate();
        let local_peer_id = local_keypair.peer_id();
        let (node, peer_id, peer_info) = Self::create_node(local_peer_id, local_keypair).await?;
        Ok((node, peer_id, peer_info))
    }

    /// Create a node from an existing keypair.
    pub async fn from_existing(mnemonic: &str) -> Result<(Self, PeerId, PeerInfo)> {
        let keypair = KeyPair::from_mnemonic(mnemonic).unwrap();
        let local_peer_id = PeerId::random();
        let (node, peer_id, peer_info) = Self::create_node(local_peer_id, keypair).await?;
        Ok((node, peer_id, peer_info))
    }

    /// Get the starknet address represented by this node.
    pub async fn starknet_address(&self) -> String {
        let chain_context = self.state_service.context.read().await;
        chain_context.address()
    }

    /// Returns the encrypted solution for the task id.
    pub async fn solution_encrypted(
        &self,
        task_id: String,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .mem_db
            .lock()
            .await
            .get_work_submission(&task_id)
            .unwrap_or(None))
    }

    pub async fn list_task(&self) -> Vec<WorkBroadcast> {
        match self.mem_db.lock().await.broadcasted_work() {
            Ok(task) => task,
            Err(_) => Vec::new()
        }
    }

    /// Notify the state service to start the syncing process.
    pub async fn sync_chain(&self) -> anyhow::Result<()> {
        // Start sync through state service
        self.state_service.start_sync().await;
        Ok(())
    }

    /// Creates a task on chain
    /// # Arguments
    /// task The task data to record on chain
    /// 
    /// # Returns
    /// transaction_hash Felt transaction hash if the transaction returned successful
    pub async fn create_task(
        &self,
        task: &FlattenedWork,
    ) -> Result<Felt, Box<dyn Error>> {
        let invoke_result = self
            .state_service
            .context
            .write()
            .await
            .create_task(task)
            .await?;

        Ok(invoke_result.transaction_hash)
    }
    
    /// Returns all solutions for a group of active task.
    pub async fn active_work_with_solutions(
        &self,
        work_group: Vec<ActiveWork>,
    ) -> Option<HashMap<String, Option<Felt>>> {
        self.mem_db
            .lock()
            .await
            .get_work_submission_hashes(&work_group)
    }

    /// Verifies the onchain encrypted solution and completes the
    /// agreement
    pub async fn verify_and_complete_solution(
        &self,
        work_id: String,
        solution_hash: Felt,
    ) -> Result<(), Box<dyn Error>> {
        self.state_service
            .context
            .write()
            .await
            .verify_and_complete(work_id, solution_hash)
            .await
    }

    /// Returns a list of [`WorkBroadcast`] from the in memory
    /// data.
    /// 
    /// # Returns
    /// task A vector of [`WorkBroadcast`].
    pub async fn list_work_opportunities(&self) -> Result<Vec<WorkBroadcast>> {
        
        let work_broadcast_json = r#"
{
    "work": {
        "id": "work_123",
        "proposal_id": null,
        "proposal_signatures": null,
        "details": {
            "description": "Example work task",
            "requirements": ["Python", "Machine Learning"],
            "expiry_date": null,
            "reward": 1000,
            "status": "Good"
        },
        "worker_address": null,
        "employer_address": "0x1234567890abcdef"
    },
    "peer_info": {
        "peer_id": "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
        "connected_addr": "/ip4/127.0.0.1/tcp/8080",
        "last_seen": 1682956800,
        "is_dedicated": false
    }
}"#;

let work_broadcast: WorkBroadcast = serde_json::from_str(work_broadcast_json)?;
let mut a = Vec::new();
a.push(work_broadcast);

Ok(a)

        // match self.mem_db.lock().await.get_work() {
        //     Ok(work_opportunities) => Ok(work_opportunities),
        //     Err(_) => {
        //         log_warning(format!("Failed to list work opportunities")).await;
        //         Ok(Vec::new())
        //     }
        // }
    }

    /// Monitors various aspects of the node, primarily syncing.
    #[instrument]
    pub async fn monitor(&mut self) {
        let mut sync_interval = tokio::time::interval(Duration::from_secs(12));
        let mut consecutive_failures = 0;
        let mut shutdown = Box::pin(tokio::signal::ctrl_c());
        
        info!("Starting monitor loop");
        loop {
            tokio::select! {
                _ = sync_interval.tick() => {
                    debug!("Starting sync interval");
                    match self.sync_chain().await {
                        Ok(_) => {
                            if consecutive_failures > 0 {
                                consecutive_failures = 0;
                                sync_interval = tokio::time::interval(Duration::from_secs(12));
                                info!("Reset sync interval after recovery");
                            }
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            let backoff_secs = std::cmp::min(12 * 2_u64.pow(consecutive_failures), 300);
                            sync_interval = tokio::time::interval(Duration::from_secs(backoff_secs));
                            error!("Sync failed with backoff {}: {}", backoff_secs, e);
                        }
                    }
                }
                _ = &mut shutdown => {
                    info!("Received shutdown signal, stopping monitor");
                    break;
                }
            }
        }
    }

    /// Initial setup for the node includes the following task:
    /// 1. Start a synchronization thread
    /// 2. Start listening on all network interfaces
    /// 3. Bootstrapping node with predefined list
    pub async fn initial_setup(&mut self) -> Result<(), Box<dyn Error>> {
        let setup_span = span!(Level::TRACE, "initial_setup");
        let _ = setup_span.enter();

        // Setup network listening on all network interfaces
        let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
   

        let state_service = self.state_service.clone();
        // Spawn the initial syncing processing in a worker thread
        tokio::spawn(async move {
            state_service.start_sync().await;
            info!("Initial synchronization step finished");
        });

        let network = &self.network;
        let  _  = network.start_listening(addr.clone()).await;
        info!("Listening on all network interfaces");

        for node in BOOTSTRAP_NODES.iter() {
         
            let update = network
                .add_peer_to_kademlia_dht(&node.peer_id, node.addr.parse().unwrap())
                .await?;


            match update {
                RoutingUpdate::Success => {
                    debug!("Bootstraping node with address {} and current state {}", node.peer_id, "success".to_string());
                }
                RoutingUpdate::Failed => {
                    debug!("Bootstraping node with address {} and current state {}", node.peer_id, "failed".to_string());
                }
                RoutingUpdate::Pending => {
                    debug!("Bootstraping node with address {} and current state {}", node.peer_id, "pending".to_string());
                }
            }
        }

        network.bootstrap_kademlia().await;
        let _joined = network.join_gossip_protocol().await?;

        info!("Initial setup complete");

        Ok(())
    }

    /// Returns the PeerInformation for this node.
    pub async fn peer_info(&self) -> PeerInfo {
        self.network.peer_info().await.unwrap()
    }

    /// Returns the local peer id for this node.
    pub async fn local_peer_id(&self) -> PeerId {
        self.network.local_peer_id().await
    }

    pub async fn list_active_work(&self) -> Result<Vec<ActiveWork>> {
        let works_future = { self.mem_db.lock().await.get_active_work() };

        match works_future {
            Ok(works) => Ok(works),
            Err(_) => Ok(Vec::new()),
        }
    }

    pub async fn get_active_work_by_id(&self, work_id: String) -> Result<Option<ActiveWork>> {
        self.mem_db.lock().await.get_active_work_by_id(work_id)
    }
    
    pub async fn list_proposals(&self) -> Result<Vec<Negotiation>> {
        match self.mem_db.lock().await.get_all_proposals().await {
            Ok(work_opportunities) => Ok(work_opportunities),
            Err(_) => {
                log_warning(format!("Failed to list work opportunities")).await;
                Ok(Vec::new())
            }
        }
    }

    /// Creates a new labor market node
    #[instrument]
    async fn create_node(
        local_peer_id: PeerId,
        local_key: KeyPair,
    ) -> Result<(Node, PeerId, PeerInfo)> {
        let configuration_exporter = ConfigurationExporter::new()?;
        let rpc = configuration_exporter.config.rpc.rpc.clone();
        let chain_id = configuration_exporter.config.rpc.chain_id.clone();
        let conode_contract_address = configuration_exporter.config.contract.conode.clone();
        let payment_token_contract_address =
            configuration_exporter.config.contract.payment_token.clone();
        let conode_deployment_block_num = configuration_exporter.config.contract.conode_deployment_block;

        info!("RPC configuration set to {}", rpc);
        info!("Connecting to Starknet chain: {}", chain_id);
        info!("Using CoNode protocol deployed at {} on block {}", conode_contract_address, conode_deployment_block_num);
        info!("Using payment token deployed at: {}", payment_token_contract_address);
        
        let signing_key = SigningKey::from_secret_scalar(Felt::from_bytes_be_slice(
            local_key.keypair().secret().as_ref(),
        ));
        let wallet = LocalWallet::from_signing_key(signing_key);

        // DistributedBehaviour (Swarm behavior)
        let behaviour =
            DistributedBehaviour::new(local_peer_id, local_key.keypair().clone().into());
        // Swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key.keypair().clone().into())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .context("Failed to create TCP transport")?
            .with_behaviour(|_| Ok(behaviour))
            .context("Failed to add behaviour to swarm")?
            .build();

        let local_peer_id = swarm.local_peer_id().clone();
        info!("Node created with PeerID {}", local_peer_id);
        let chain_config = ChainConfig {
            rpc,
            chain_id,
            conode_contract: conode_contract_address,
            payment_token_contract: payment_token_contract_address,
            deployed_block_num: conode_deployment_block_num,
        };

        let chain_context = Arc::new(RwLock::new(
            ChainContext::new(wallet, local_key.starknet_address, chain_config.clone()).unwrap(),
        ));

        let mem_db = Arc::new(Mutex::new(InMemoryDb::new()?));

        let (sync_event_sender, sync_event_receiver) =
            tokio::sync::watch::channel(SyncEvent::SyncCompleted);
        let sync_state = SyncState::new(Arc::clone(&mem_db), chain_context.clone(), chain_config, sync_event_sender.clone());

        let arc_swarm = Arc::new(Mutex::new(swarm));

        let (handle, mut network_rx) = Network::new(
            arc_swarm.clone(),
            local_key.clone(),
            chain_context.clone(),
            Arc::clone(&mem_db),
            configuration_exporter.clone(),
            RequestHandler::new(arc_swarm.clone(), mem_db.clone(),Box::new(DiskDb::default()), RetryConfig::default()),
            Box::new(DiskDb::default())
        ).await?;

        let local_peer_info = handle.peer_info().await.unwrap().clone();
        info!("Node created with peer info: {:?}", local_peer_info);

        let node = Node {
            network: handle,
            local_peer_id,
            state_service: Arc::new(StateService {
                sync_state: Arc::new(RwLock::new(sync_state)),
                event_rcv: sync_event_receiver.clone(),
                context: chain_context,
            }),
            mem_db,
            sync_event_receiver
        };

        Ok((node, local_peer_id, local_peer_info))
    }
}

impl Drop for Node {
    fn drop(&mut self) {}
}
