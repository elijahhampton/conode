use anyhow::{Context, Result};
use conode_config::bootnode::BOOTSTRAP_NODES;
use conode_logging::logger::{log_info, log_warning};
use conode_network::behaviour::DistributedBehaviour;
use conode_network::handle::NetworkHandle;
use conode_network::network::CoNodeNetwork;
use conode_starknet::crypto::keypair::KeyPair;
use conode_storage::state::{create_shared_state, SharedStateArc};
use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::work::{ActiveWork, FlattenedWork, WorkBroadcast};
use libp2p::{Multiaddr, PeerId, SwarmBuilder};
use log::{debug, error, info};
use starknet::core::types::{Felt, InvokeTransactionResult};
use starknet::signers::{LocalWallet, SigningKey};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{event, span, Level};

use super::event::{NetworkEvent, NodeCommand};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{interval, Duration};

/// A shareable interface and frontend to interact with CoNode.
#[derive(Clone)]
pub struct LaborMarketNode {
    // A handle to interface with the network
    pub network: NetworkHandle,
    // A command receivier for external [NodeCommand]s.
    pub command_receiver: Arc<TokioMutex<mpsc::Receiver<NodeCommand>>>,
    // An event sender for [NetworkEvent]s.
    event_sender: mpsc::Sender<NetworkEvent>,
    #[allow(dead_code)] // Runtime is used during create_node
    runtime: Arc<tokio::runtime::Runtime>,
    // A shared state with the Network
    shared_state: SharedStateArc,
}

impl fmt::Debug for LaborMarketNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaborMarketNode")
            .field("network", &"NetworkHandle")
            .field("command_receiver", &"mpsc::Receiver<NodeCommand>")
            .field("event_sender", &"mpsc::Sender<NetworkEvent>")
            .field("runtime", &"tokio::runtime::Runtime")
            .finish()
    }
}

impl LaborMarketNode {
    /// Creates a new instance of the LaborMarketNode.
    pub async fn new() -> Result<(
        Self,
        mpsc::Sender<NodeCommand>,
        mpsc::Receiver<NetworkEvent>,
        PeerId,
        PeerInfo,
    )> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap(),
        );

        let local_keypair = KeyPair::generate();
        let local_peer_id = local_keypair.peer_id();

        let result = runtime.block_on(LaborMarketNode::create_node(
            local_peer_id,
            local_keypair,
            runtime.clone(),
        ));

        log_info("Starting LaborMarketNode".to_string()).await;
        Ok(result?)
    }

    /// Creates a new instance of the LaborMarketNode using a libp2p and app seed for the keypair.
    pub async fn from_existing(
        mnemonic: &str,
    ) -> Result<(
        Self,
        mpsc::Sender<NodeCommand>,
        mpsc::Receiver<NetworkEvent>,
        PeerId,
        PeerInfo,
    )> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap(),
        );

        let keypair = KeyPair::from_mnemonic(mnemonic).unwrap();
        let local_peer_id = PeerId::random();

        let result = runtime.block_on(LaborMarketNode::create_node(
            local_peer_id,
            keypair,
            runtime.clone(),
        ));

        log_info("Starting LaborMarketNode".to_string()).await;
        Ok(result?)
    }

    pub async fn solution(
        &self,
        work_id: String,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let state = self.shared_state.lock().await;
        Ok(state
            .storage_manager
            .get_work_submission(&work_id)
            .unwrap_or(None))
    }

    pub async fn active_work_with_solutions(
        &self,
        work_group: Vec<ActiveWork>,
    ) -> Option<HashMap<String, Option<Felt>>> {
        self.shared_state
            .lock()
            .await
            .storage_manager
            .get_work_submission_hashes(&work_group)
    }

    async fn sync_chain(&self) -> anyhow::Result<()> {
        let mut state = self.shared_state.lock().await;
        let _ = state.sync_manager.sync_to_latest_block().await;
        Ok(())
    }

    pub async fn starknet_address(&self) -> String {
        self.shared_state.lock().await.chain_manager.address()
    }

    /// Returns a list of `WorkBroadcast` stored in the underlying database
    pub async fn list_work_opportunities(&self) -> Result<Vec<WorkBroadcast>> {
        match self.shared_state.lock().await.storage_manager.get_work() {
            Ok(work_opportunities) => Ok(work_opportunities),
            Err(_) => {
                log_warning(format!("Failed to list work opportunities")).await;
                Ok(Vec::new())
            }
        }
    }

    pub async fn list_active_work(&self) -> Result<Vec<ActiveWork>> {
        let works_future = {
            let state_lock = self.shared_state.lock().await;
            state_lock.storage_manager.get_active_work()
        };

        match works_future {
            Ok(works) => Ok(works),
            Err(_) => Ok(Vec::new()),
        }
    }

    pub async fn get_active_work_by_id(&self, work_id: String) -> Result<Option<ActiveWork>> {
        let state_lock = self.shared_state.lock().await;
        state_lock.storage_manager.get_active_work_by_id(work_id)
    }

    /// Starts the node, handles local setup and begins listening for NodeCommands,
    /// and various swarm events.
    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Starting LaborMarketNode with PeerId: {}",
            self.local_peer_id().await
        );

        self.initial_setup().await;
        debug!("Completed initial setup.");
        let mut sync_interval = interval(Duration::from_secs(10));

        loop {
            let command = {
                let mut receiver = self.command_receiver.lock().await;

                tokio::select! {
                    Some(cmd) = receiver.recv() => cmd,
                    _ = sync_interval.tick() => {
                        if let Err(e) = self.sync_chain().await {
                            error!("Chain sync failed: {:?}", e);
                        }
                        continue;
                    }
                }
            };

            if let NodeCommand::Stop(sender) = command {
                info!("Received stop command. Shutting down...");
                let _ = sender.send(());
                break;
            }
            if let Err(e) = self.handle_command(command).await {
                error!("Failed to execute command: {:?}", e);
            }
        }

        Ok(())
    }

    /// Handles commands received from an external interface.
    async fn handle_command(
        &mut self,
        command: NodeCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            NodeCommand::BroadcastJob(job) => {
                let work_id = job.work.id.clone();

                self.network.broadcast_work(job).await;

                self.event_sender
                    .send(NetworkEvent::JobBroadcastConfirmed(work_id.to_string()))
                    .await?;

                return Ok(());
            }
            NodeCommand::InitiateNegotiation(peer_id, job_id, negotiation) => {
                self.network
                    .initiate_negotiation(peer_id, job_id.clone(), negotiation)
                    .await;

                self.event_sender
                    .send(NetworkEvent::NegotiationInitiated(peer_id, job_id))
                    .await?;
            }
            NodeCommand::Jobs => todo!(),
            NodeCommand::SubscribeToTopic(topic) => {
                self.network.subscribe(topic.clone()).await;
                self.event_sender
                    .send(NetworkEvent::TopicSubscribed(topic))
                    .await?;
            }
            NodeCommand::UnsubscribeFromTopic(topic) => {
                self.network.unsubscribe(topic.clone()).await;
                self.event_sender
                    .send(NetworkEvent::TopicUnsubscribed(topic))
                    .await?;
            }
            NodeCommand::Stop(_) => {
                self.network.shutdown().await?;
                self.event_sender
                    .send(NetworkEvent::ShutdownComplete)
                    .await?;
            }
            NodeCommand::Topics => {}
            NodeCommand::CreateTopic(_topic) => {}
        }
        Ok(())
    }

    /// Starts swarm listening on all network interfaces over TCP.
    async fn initial_setup(&mut self) {
        let setup_span = span!(Level::TRACE, "initial_setup");
        let _ = setup_span.enter();

        // Setup network listening on all network interfaces
        let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();

        if let Err(e) = self.sync_chain().await {
            panic!("Initial chain sync failed: {:?}", e);
        }

        // Start listening
        match self.network.start_listening(addr.clone()).await {
            Ok(_) => {
                event!(Level::TRACE, "Network listener started successfully");
            }
            Err(_e) => panic!("Network failed to start and listen on {}", addr.to_string()),
        }
        event!(Level::TRACE, "Listening on all network interfaces");

        // Connect to bootstrap nodes
        for node in BOOTSTRAP_NODES.iter() {
            debug!("Adding bootstrap node: {}", node.peer_id);
            self.network
                .add_peer_to_kademlia_dht(&node.peer_id, node.addr.parse().unwrap())
                .await;
        }
        event!(Level::TRACE, "Bootstrapped all bootstrap nodes.");

        // Bootstrap Kademlia
        info!("Bootstrapping Kademlia DHT");
        self.network.bootstrap_kademlia().await;

        // Join gossip protocol
        info!("Joining gossip protocol");
        match self.network.join_gossip_protocol().await {
            Ok(true) => {
                log_info("Your node is now listening for work broadcast!".to_string()).await
            }
            Ok(false) => log_warning(
                "Your node failed to start listening for work broadcast. Retrying in 30 seconds..."
                    .to_string(),
            )
            .await,
            Err(_e) => log_warning(
                "Your node failed to start listening for work broadcast. Retrying in 30 seconds..."
                    .to_string(),
            )
            .await,
        }
        event!(Level::TRACE, "Joined gossipsub work_discovery topic");
    }

    /// Returns the PeerInformation for this node.
    pub async fn peer_info(&self) -> PeerInfo {
        self.network.peer_info().await.unwrap()
    }

    /// Returns the local peer id for this node.
    pub async fn local_peer_id(&self) -> PeerId {
        self.network.local_peer_id().await
    }

    pub async fn create_work(
        &self,
        work: &FlattenedWork,
    ) -> Result<InvokeTransactionResult, Box<dyn Error>> {
        let state_lock = self
            .shared_state
            .lock()
            .await
            .chain_manager
            .create_work(work)
            .await;
        state_lock
    }

    pub async fn verify_and_complete_solution(
        &self,
        work_id: String,
        solution_hash: Felt,
    ) -> Result<(), Box<dyn Error>> {
        self.shared_state
            .lock()
            .await
            .chain_manager
            .verify_and_complete(work_id, solution_hash)
            .await
    }

    pub async fn list_proposals(&self) -> Result<Vec<Negotiation>> {
        let state = self.shared_state.lock().await;

        match state.storage_manager.get_all_proposals().await {
            Ok(work_opportunities) => Ok(work_opportunities),
            Err(_) => {
                log_warning(format!("Failed to list work opportunities")).await;
                Ok(Vec::new())
            }
        }
    }

    /// Creates a new labor market node
    async fn create_node(
        local_peer_id: PeerId,
        local_key: KeyPair,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Result<(
        LaborMarketNode,
        mpsc::Sender<NodeCommand>,
        mpsc::Receiver<NetworkEvent>,
        PeerId,
        PeerInfo,
    )> {
        // Create channels
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create wallet
        let key = SigningKey::from_secret_scalar(Felt::from_bytes_be_slice(
            local_key.keypair().secret().as_ref(),
        ));

        let wallet = LocalWallet::from_signing_key(key);

        // Create behavior
        let behaviour =
            DistributedBehaviour::new(local_peer_id, local_key.keypair().clone().into());

        // Build swarm
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

        // Create shared state
        let shared_state = create_shared_state(wallet, local_peer_id, local_key.starknet_address)?;

        // Create network
        let (handle, mut network_rx) =
            CoNodeNetwork::new(swarm, shared_state.clone(), local_key.clone()).await?;

        let handle_clone = handle.clone();

        let local_peer_info = handle.peer_info().await.unwrap().clone();

        runtime.spawn(async move {
            while let Some(msg) = network_rx.recv().await {
                handle_clone.handle_message(msg).await;
            }
        });

        let node = LaborMarketNode {
            network: handle,
            command_receiver: Arc::new(TokioMutex::new(command_rx)),
            event_sender: event_tx,
            runtime,
            shared_state,
        };

        Ok((node, command_tx, event_rx, local_peer_id, local_peer_info))
    }
}

impl Drop for LaborMarketNode {
    fn drop(&mut self) {
        // Create a new runtime for cleanup if main runtime is already shut down
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            if let Err(e) = self.network.shutdown().await {
                log::error!("Error shutting down network during drop: {:?}", e);
            }

            if let Err(e) = self.event_sender.send(NetworkEvent::ShutdownComplete).await {
                log::error!("Error sending shutdown event during drop: {:?}", e);
            }
        });
    }
}
