use std::borrow::Borrow;
use std::future::Future;
use std::str::FromStr;
use std::{error::Error, sync::Arc};

use crate::behaviour::DistributedBehaviour;
use crate::connection::{ListenerCloseReason, NetworkConnectionHandler};
use crate::event::{CoNodeEvent, DistributedBehaviourEvent};
use crate::peer::peer_connection::{PeerConnection, PeerManager};
use crate::protocol::{
    discovery::{
        traits::{Discoverer, Seeker},
        work_discovery_topic,
    },
    negotiation::traits::Negotiator,
};

use chrono::DateTime;
use conode_logging::logger::{log_info, log_warning};
use conode_starknet::crypto::keypair::KeyPair;
use conode_storage::state::{SharedState, SharedStateArc};
use conode_types::crypto::{ECDSASignature, FeltWrapper};
use conode_types::negotiation::ProposalStatus;
use conode_types::peer::PeerInfo;

use conode_types::traits::libp2p::GossipsubNodeProvider;
use conode_types::work::{ActiveWork, JobRole};
use conode_types::{
    negotiation::{Negotiation, NegotiationRequest, NegotiationResponse},
    work::{FlattenedWork, Work, WorkBroadcast},
};
use libp2p::gossipsub::SubscriptionError;
use libp2p::request_response::Event as RequestResponseEvent;
use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use libp2p::{
    gossipsub::Message as GossipsubMessage,
    request_response::{InboundRequestId, OutboundRequestId, ResponseChannel},
    PeerId, Swarm,
};
use log::{debug, error, info};
use rand::{self, Rng};

use starknet::core::crypto::pedersen_hash;
use starknet::core::types::Felt;

use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::handle::{NetworkHandle, NetworkHandleMessage};

/// Provides the core network implementation of the protocol. All functionality related to
/// related to peer interaction (Gossip, Kademlia, RequestResponse and all distributed behaviours) is
/// handled by the CoNodeNetwork.
pub struct CoNodeNetwork {
    pub swarm: Swarm<DistributedBehaviour>,
    shared_state: SharedStateArc,
    peer_manager: PeerManager,
    connection_handler: NetworkConnectionHandler,
    keypair: KeyPair,
}

impl CoNodeNetwork {
    pub async fn new(
        swarm: Swarm<DistributedBehaviour>,
        shared_state: Arc<tokio::sync::Mutex<SharedState>>,
        keypair: KeyPair,
    ) -> anyhow::Result<(NetworkHandle, mpsc::UnboundedReceiver<NetworkHandleMessage>)> {
        // @dev Note this is an unbounded mpsc channel
        let (_network_tx, network_rx) = mpsc::unbounded_channel();

        let network = Arc::new(Mutex::new(Self {
            swarm,
            shared_state: shared_state.clone(),
            peer_manager: PeerManager::new(),
            connection_handler: NetworkConnectionHandler::new(),
            keypair: keypair.clone(),
        }));

        // We return a network handle which will serve as an interface into the
        // network.
        let handle = NetworkHandle::new(network);

        Ok((handle, network_rx))
    }

    // Handles events emitted from libp2p swarm.
    pub async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent<DistributedBehaviourEvent>,
    ) -> Result<(), Box<dyn Error>> {
        match event {
            SwarmEvent::Behaviour(behaviour) => match behaviour {
                DistributedBehaviourEvent::CoNode(conode_event) => {
                    self.handle_conode_event(conode_event).await?;
                }
                DistributedBehaviourEvent::Identify(event) => match event {
                    libp2p::identify::Event::Received {
                        connection_id: _,
                        peer_id,
                        info,
                    } => {
                        self.peer_manager.add_peer_identity(&peer_id, info).await;
                    }
                    libp2p::identify::Event::Sent {
                        connection_id: _,
                        peer_id,
                    } => {
                        debug!(
                            "[libp2p::identify::Event::Sent]: Sent identity to peer {}",
                            peer_id.to_string()
                        );
                    }
                    libp2p::identify::Event::Pushed {
                        connection_id: _,
                        peer_id,
                        info,
                    } => {
                        self.peer_manager.new_identification(peer_id, info).await;
                    }
                    libp2p::identify::Event::Error {
                        connection_id: _,
                        peer_id,
                        error,
                    } => {
                        error!("[libp2p::identify::Event::Error]: Error while trying to identify remote {}. {}", peer_id.to_string(), error.to_string());
                    }
                },
                DistributedBehaviourEvent::AutoNat(_event) => {
                    info!("[AutoNat::Event]: For future use");
                } // DistributedBehaviourEvent::Relay(_) => todo("For future use"),
                  // DistributedBehaviourEvent::Dcutr(_) => todo("For future user")
            },
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established: _,
                concurrent_dial_errors: _,
                established_in: _,
            } => {
                let peer_connection = PeerConnection::new(connection_id, endpoint);

                self.peer_manager
                    .new_peer_connection(peer_id, peer_connection)
                    .await;
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id: _,
                endpoint: _,
                num_established: _,
                cause,
            } => {
                self.peer_manager
                    .close_peer_connection(&peer_id, cause)
                    .await;
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                self.connection_handler
                    .new_listen_addr(listener_id, address)
                    .await;
            }
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                // Expire the address
                self.connection_handler
                    .expire_address(listener_id, &address)
                    .await;

                // Attempt to reconnect
                if let Ok(listener_id) = self.swarm.listen_on(address.clone()) {
                    self.connection_handler
                        .new_listen_addr(listener_id, address)
                        .await;
                }
            }
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses: _,
                reason: _,
            } => {
                let _ = self
                    .connection_handler
                    .close_listener(listener_id, ListenerCloseReason::Done)
                    .await;
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                let _ = self
                    .connection_handler
                    .set_listener_error(listener_id, error)
                    .await;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles events related to the CoNode protocol including: Gossipsub and RequestResponse.
    async fn handle_conode_event(&mut self, event: CoNodeEvent) -> Result<(), Box<dyn Error>> {
        match event {
            CoNodeEvent::NegotiationRequestResponse(event) => match event {
                RequestResponseEvent::Message { peer, message } => match message {
                    libp2p::request_response::Message::Request {
                        request_id,
                        request,
                        channel,
                    } => {
                        self.handle_negotiation_request(peer, request_id, request, channel)
                            .await;
                    }
                    libp2p::request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        self.handle_negotiation_response(request_id, response).await;
                    }
                },
                _ => {}
            },
            CoNodeEvent::Kademlia(event) => {
                // Handle all Kad events through the peer manager
                self.peer_manager.handle_kad_event(event).await;
            }
            CoNodeEvent::Mdns(event) => {
                let event_clone = event.clone();

                // First handle connections in the peer manager
                self.peer_manager.handle_mdns_event(event).await;

                // Update kademlia if needed
                match event_clone {
                    libp2p::mdns::Event::Discovered(discovered_nodes) => {
                        for node in discovered_nodes.clone().iter() {
                            self.swarm
                                .behaviour_mut()
                                .conode
                                .kademlia
                                .add_address(&node.0, node.1.clone());
                        }
                    }
                    libp2p::mdns::Event::Expired(expired_nodes) => {
                        for node in expired_nodes.iter() {
                            self.swarm
                                .behaviour_mut()
                                .conode
                                .kademlia
                                .remove_address(&node.0, &node.1.clone());
                        }
                    }
                }
            }
            CoNodeEvent::Gossipsub(event) => match event {
                libp2p::gossipsub::Event::Message {
                    propagation_source: _,
                    message_id: _,
                    message,
                } => {
                    let _ = self.handle_gossipsub_message(message).await;
                }
                libp2p::gossipsub::Event::Subscribed { peer_id, topic } => {
                    info!(
                        "[new peer subscription] peer {} subcribed to topic {}",
                        peer_id.to_string(),
                        topic.into_string()
                    );
                }
                libp2p::gossipsub::Event::Unsubscribed { peer_id, topic } => {
                    info!(
                        "peer {} unsubscribed from topic {}",
                        peer_id.to_string(),
                        topic.into_string()
                    );
                }
                _ => {}
            },
        }
        Ok(())
    }

    /// Returns the peer info for this network
    pub fn peer_info(&self) -> PeerInfo {
        let connected_addr = Some(
            self.swarm
                .listeners()
                .next()
                .map(|addr| addr.clone())
                .unwrap_or(Multiaddr::empty()),
        );
        PeerInfo {
            peer_id: self.swarm.local_peer_id().clone(),
            connected_addr: connected_addr,
            last_seen: DateTime::from_timestamp_nanos(0),
            is_dedicated: false,
        }
    }

    /// Shutdown the network.
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        // Close all listeners
        let listener_ids: Vec<_> = self
            .connection_handler
            .listener_statuses
            .lock()
            .await
            .keys()
            .cloned()
            .collect();

        for id in listener_ids {
            self.swarm.remove_listener(id);
        }

        let peers: Vec<_> = self.swarm.connected_peers().cloned().collect();

        // Disconnect and remove all peers from the KADHT
        for peer in peers {
            if let Err(e) = self.swarm.disconnect_peer_id(peer) {
                log::warn!("Failed to disconnect from peer {}: {:?}", peer, e);
            }

            self.swarm
                .behaviour_mut()
                .conode
                .kademlia
                .remove_peer(&peer);
        }

        // Clear any pending messages/state
        self.connection_handler.clear_state().await;

        log::info!("Network shutdown completed");
        Ok(())
    }
}

impl Discoverer for CoNodeNetwork {
    /// Checks to see if potential work items from `WorkBroadcast` match the nodes criteria. This function
    /// acts as a filtering mechanism storing only eligible work items.
    fn handle_potential_work(
        &mut self,
        work: &WorkBroadcast,
        _message: &GossipsubMessage,
    ) -> impl std::future::Future<Output = Result<bool, Box<dyn Error>>> + Send {
        async move {
            if !self.is_matching_node_critera(&work.work) {
                return Ok(false);
            }

            // Eventually this will need to batch multiple work items.
            let shared_state = self.shared_state.lock().await;
            let _ = shared_state
                .storage_manager
                .add_potential_work(work.clone());

            Ok(true)
        }
    }

    /// Returns true or false based on how the node defines
    /// eligible `Work`.
    fn is_matching_node_critera(&self, _work: &Work) -> bool {
        true
    }
}

impl Seeker for CoNodeNetwork {
    /// Broadcast work to the network under the work discovery topic. Peers will have access to
    /// work information and peer info.
    fn broadcast_work<'a>(
        &'a mut self,
        data: &'a WorkBroadcast,
    ) -> impl std::future::Future<Output = bool> + Send + 'a {
        async move {
            let publish_result = self
                .swarm
                .behaviour_mut()
                .conode
                .gossipsub
                .publish(work_discovery_topic(), bincode::serialize(data).unwrap());

            match publish_result {
                Ok(_) => {
                    // Add the successfully broadcasted item to the db
                    let _ = self
                        .shared_state
                        .lock()
                        .await
                        .storage_manager
                        .add_broadcasted_work(data.work.clone());

                    true
                }
                Err(err) => {
                    error!("[gossipsub publishing error]: {:?}", err.to_string());
                    log_warning(format!(
                        "[gossipsub publishing error]: {:?}",
                        err.to_string()
                    ))
                    .await;

                    false
                }
            }
        }
    }
}

impl Negotiator for CoNodeNetwork {
    /// Initiate a negotiation with a peer.
    async fn initiate_negotiation(
        &mut self,
        recipient: &PeerId,
        _job_id: &String,
        negotiation: &mut Negotiation,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Set the correct negotiation status
        assert!(negotiation.status == None);
        negotiation.status = Some(ProposalStatus::Proposed);

        // Create a new proposal
        let new_proposal_request = NegotiationRequest::NewProposal(
            self.swarm.local_peer_id().clone(),
            negotiation.clone(),
        );

        let _ = self
            .shared_state
            .lock()
            .await
            .storage_manager
            .save_or_update_proposal(&negotiation)
            .await;

        self.swarm
            .behaviour_mut()
            .conode
            .negotiation_protocol
            .send_request(recipient, new_proposal_request);

        Ok(())
    }

    /// Request a completion acknowledgement for a negotiation.
    /// Sign the negotiation and send an EDCSA keypair in order to recover the public key on chain
    async fn request_completion_ack(&mut self, recipient: &PeerId, mut negotiation: Negotiation) {
        let (signature, _storage_result) = {
            let shared_state = self.shared_state.lock().await;
            let negotiation_hash = Felt::from_bytes_be_slice(negotiation.id.as_bytes());

            // Run both operations under one lock
            let signature = shared_state
                .chain_manager
                .sign(self.keypair.stark_private_key(), &negotiation_hash)
                .await;

            match signature {
                Some(signature) => {
                    let ecdsa_signature = ECDSASignature {
                        r: signature.r.into(),
                        s: signature.s.into(),
                        v: signature.v.into(),
                    };

                    negotiation.employer_signature = Some(ecdsa_signature.clone());
                    negotiation.status =
                        Some(ProposalStatus::EmployerSigned(ecdsa_signature.clone()));

                    let storage_result = shared_state
                        .storage_manager
                        .save_or_update_proposal(&negotiation)
                        .await
                        .map_err(|e| e.to_string());

                    if let Err(storage_result) = storage_result {
                        log_warning(format!("Failed to send completion request acknowledgement. Unable to save or update proposal. {:?}", storage_result.to_string())).await;
                        return;
                    }

                    (ecdsa_signature, storage_result)
                }
                None => {
                    log_warning(format!("Failed to send completion request acknowledgement")).await;
                    return;
                }
            }
        };

        let request_completion_ack_request = NegotiationRequest::RequestCompletionAcknowledgement(
            self.swarm.local_peer_id().clone(),
            negotiation.id.clone(),
            self.keypair.stark_public_key().clone().into(),
            signature,
        );

        let _outbound_request_id = self
            .swarm
            .behaviour_mut()
            .conode
            .negotiation_protocol
            .send_request(recipient, request_completion_ack_request);
    }

    /// Send a completion confirmation request to a peer for a negotiation.
    /// @dev On chain transaction occurs here
    async fn send_completion_confirmation_request(
        &mut self,
        recipient: &PeerId,
        negotiation_id: &String,
    ) {
        let shared_state = self.shared_state.lock().await;
        let negotiation = shared_state.storage_manager.get_proposal(&negotiation_id);

        if let Some(active_negotiation) = negotiation {
            // We unwrap the address and signatures or simply assign None, expecting the
            // smart contract to correctly panic and the transaction to fail at this point
            // if the negotiation does not contain a worker address
            let employer_signature =
                active_negotiation
                    .employer_signature
                    .unwrap_or(ECDSASignature {
                        r: FeltWrapper::from(Felt::ZERO),
                        s: FeltWrapper::from(Felt::ZERO),
                        v: FeltWrapper::from(Felt::ZERO),
                    });
            let worker_signature = active_negotiation
                .worker_signature
                .unwrap_or(ECDSASignature {
                    r: FeltWrapper::from(Felt::ZERO),
                    s: FeltWrapper::from(Felt::ZERO),
                    v: FeltWrapper::from(Felt::ZERO),
                });
            let worker_address = active_negotiation
                .worker_address
                .unwrap_or(FeltWrapper::from(Felt::ZERO));

            // Create and assign the new work a unique id
            let work_id = Uuid::new_v4().to_string();
            let created_work_data = FlattenedWork {
                id: work_id,
                proposal_signatures: vec![employer_signature, worker_signature],
                expiry_date: 0,
                reward: active_negotiation.proposed_payout.unwrap_or(0) as u64,
                worker_address,
            };

            let active_work_entry = ActiveWork {
                role: JobRole::Employer,
                work: created_work_data.clone(),
                employer_peer_id: self.peer_info().peer_id,
                worker_peer_id: recipient.to_owned(),
            };

            if shared_state
                .storage_manager
                .store_active_work(active_work_entry)
                .is_err()
            {
                return;
            }

            // We go ahead and send the request for the active work item to the peer. Here one of two
            // things can go wrong. An error can occur on the `worker's` side and this function will bail here
            // or the request is sent successfully, but the transaction sent to the chain fails. For this reason we should send
            // the request here and wait for the appropriate acknowledgement/response to return successful before submitting the transaction
            // to the chain.
            let _request = self
                .swarm
                .behaviour_mut()
                .conode
                .negotiation_protocol
                .send_request(
                    recipient,
                    NegotiationRequest::CompletionConfirmation(created_work_data.clone()),
                );
        }
    }

    /// Send a completion confirmation acknowledgement with the data location of the completed
    /// work.
    /// @dev On chain transaction occurs here
    /// @dev This function potentially has the opportunity to hold
    /// the lock for a long period of time
    async fn send_completion_confirmation_ack(
        &mut self,
        to: &PeerId,
        work_id: String,
        uri: String,
    ) {
        let shared_state = self.shared_state.lock().await;

        let active_work = shared_state
            .storage_manager
            .get_active_work_by_id(work_id.clone())
            .map_err(|e| e.to_string());

        match active_work {
            Ok(work) => {
                if let Some(active_work) = work {
                    // Generate a random salt
                    let salt: u8 = rand::thread_rng().gen();
                    // Ensure the salt is encoded in hex
                    let salt_hex = format!("0x{:02x}", salt);

                    // Ensure the uri is encoded in hex
                    let uri_hex = if !uri.starts_with("0x") {
                        format!("0x{}", hex::encode(uri.clone().as_bytes()))
                    } else {
                        uri.clone()
                    };

                    // Safely convert hex strings to Felt for compatibility with Starknet
                    let uri_felt = Felt::from_str(&uri_hex).map_err(|e| e.to_string());
                    if let Err(uri_felt) = uri_felt {
                        error!(
                            "[send_completion_confirmation_ack] Failed to convert solution uri to felt: {:?}",
                            uri_felt.to_string()
                        );
                        log_warning(format!("Failed to convert solution uri to Felt.")).await;
                        return;
                    }

                    let salt_felt = Felt::from_str(&salt_hex).map_err(|e| e.to_string());
                    if let Err(salt_felt) = salt_felt {
                        error!(
                            "[send_completion_confirmation_ack] Failed to convert solution uri to felt: {:?}",
                            salt_felt.to_string()
                        );
                        log_warning(format!("Failed to convert solution uri to Felt.")).await;
                        return;
                    }

                    // Compute the pederson_hash of the uri and salt
                    let p_hash = pedersen_hash(&uri_felt.unwrap(), &salt_felt.unwrap());

                    // Store the computed salt and work item
                    let salt_storage_result = self
                        .shared_state
                        .lock()
                        .await
                        .storage_manager
                        .store_work_salt(work_id.clone(), salt.to_string())
                        .map_err(|e| e.to_string());

                    if let Err(salt_storage_result) = salt_storage_result {
                        error!(
                            "[send_completion_confirmation_ack] Failed to carry out storage of solution salt: {:?}",
                            salt_storage_result.to_string()
                        );
                        log_warning(format!("Failed to carry out storage of solution salt.")).await;
                        return;
                    }

                    let work_storage_result = self
                        .shared_state
                        .lock()
                        .await
                        .storage_manager
                        .store_work_solution(work_id.clone(), uri.to_string())
                        .map_err(|e| e.to_string());

                    if let Err(work_storage_result) = work_storage_result {
                        error!(
                            "[send_completion_confirmation_ack] Failed to carry out storage of work: {:?}",
                            work_storage_result.to_string()
                        );
                        log_warning(format!("Failed to carry out storage of work.")).await;
                        return;
                    }

                    let transaction_result = self
                        .shared_state
                        .lock()
                        .await
                        .chain_manager
                        .submit_solution(active_work.work.id, p_hash)
                        .await
                        .map_err(|e| e.to_string());

                    // If the transaction was submitted to the chain successfully perform the
                    // network request.
                    if let Ok(_) = transaction_result {
                        self.swarm
                            .behaviour_mut()
                            .conode
                            .negotiation_protocol
                            .send_request(
                                to,
                                NegotiationRequest::CompletionAcknowledgement(
                                    work_id,
                                    uri,
                                    salt.to_string(),
                                ),
                            );
                    }
                }
            }
            Err(err) => {
                log_warning(format!(
                    "[send_completion_confirmation_ack] Unable to find work item: {:?}",
                    err.to_string()
                ))
                .await;
            }
        }
    }

    /// Request handler for [NegotiationRequest]. See [NegotiationRequest] to learn about
    /// the signifigance of each enum variant.
    async fn handle_negotiation_request(
        &mut self,
        from: PeerId,
        _request_id: InboundRequestId,
        request: NegotiationRequest,
        channel: ResponseChannel<NegotiationResponse>,
    ) {
        match request {
            NegotiationRequest::NewProposal(_from, mut negotiation) => {
                negotiation.status = Some(ProposalStatus::Acknowledged);

                let shared_state = self.shared_state.lock().await;

                match shared_state
                    .storage_manager
                    .save_or_update_proposal(&negotiation)
                    .await
                {
                    Ok(_) => {
                        let mutable_swarm = self.swarm.behaviour_mut();
                        let peer_id = self.shared_state.lock().await.local_peer_id;

                        // We don't handle the response result here as we expect an ResponseResponse::InboundError
                        // if this occurs
                        let _result = mutable_swarm.conode.negotiation_protocol.send_response(
                            channel,
                            NegotiationResponse::ProposalAcknowledgement(peer_id, negotiation.id),
                        );
                    }
                    Err(err) => {
                        log_warning(format!(
                            "Failed to save received proposal: {:?}",
                            err.to_string()
                        ))
                        .await;
                    }
                }
            }
            NegotiationRequest::RequestCompletionAcknowledgement(
                _peer_id,
                negotiation_id,
                employer_public_key,
                employer_signature,
            ) => {
                let shared_state = self.shared_state.lock().await;

                let negotiation_or_none =
                    shared_state.storage_manager.get_proposal(&negotiation_id);
                if negotiation_or_none.is_none() {
                    return;
                }

                let mut negotiation = negotiation_or_none.unwrap();

                let verification_data = Felt::from_bytes_be_slice(negotiation_id.as_bytes());
                if !shared_state.chain_manager.verify(
                    verification_data,
                    employer_public_key.into(),
                    &employer_signature,
                ) {
                    log_warning(format!(
                        "Signature verification failed for received proposal."
                    ))
                    .await;
                    error!("Signature verification failed.");
                    return;
                }

                negotiation.employer_signature = Some(employer_signature.clone());
                let negotiation_hash = Felt::from_bytes_be_slice(negotiation_id.as_bytes());

                // Use STARK private key for worker signature
                let worker_signature = shared_state
                    .chain_manager
                    .sign(self.keypair.stark_private_key(), &negotiation_hash)
                    .await;

                if worker_signature.is_none() {
                    log_warning(format!("Unsuccessful signing of proposal.")).await;
                    error!("Negotiation signing failed.");
                    return;
                }

                if let Some(worker_signature) = worker_signature {
                    let worker_signature = ECDSASignature {
                        s: worker_signature.s.into(),
                        r: worker_signature.r.into(),
                        v: worker_signature.v.into(),
                    };

                    negotiation.worker_signature = Some(worker_signature.clone());
                    negotiation.status = Some(ProposalStatus::FullySigned {
                        employer_signature,
                        worker_signature: worker_signature.clone(),
                    });

                    if shared_state
                        .storage_manager
                        .save_or_update_proposal(&negotiation)
                        .await
                        .is_err()
                    {
                        log_warning(format!(
                            "[handle_negotiation_request] Failed to save or update proposal."
                        ))
                        .await;
                        return;
                    }

                    let acknowledgement = NegotiationResponse::AcknowledgementReceived(
                        self.swarm.local_peer_id().clone(),
                        negotiation_id.clone(),
                        worker_signature.clone(),
                        shared_state.chain_manager.address_as_felt().into(),
                    );

                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .conode
                        .negotiation_protocol
                        .send_response(channel, acknowledgement);
                }
            }
            NegotiationRequest::CompletionConfirmation(work) => {
                let shared_state = self.shared_state.lock().await;
                let active_work_entry = ActiveWork {
                    work: work.clone(),
                    role: JobRole::Worker,
                    employer_peer_id: from,
                    worker_peer_id: self.peer_info().peer_id,
                };

                let worker_peer_id = active_work_entry.worker_peer_id.clone();
                if shared_state
                    .storage_manager
                    .store_active_work(active_work_entry)
                    .is_err()
                {
                    log_warning(format!(
                        "Failed to store recieved work entry from peer {}",
                        worker_peer_id
                    ))
                    .await;
                    return;
                }

                let _ = self
                    .swarm
                    .behaviour_mut()
                    .conode
                    .negotiation_protocol
                    .send_response(
                        channel,
                        NegotiationResponse::ActiveWorkAcknowledgement(work),
                    );
            }
            NegotiationRequest::CompletionAcknowledgement(_work_id, _solution_uri, _salt) => {
                // We don't need to store the solution received here. The node will listen to submision events
                // and properly updated the work submission as of the latest block.
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .conode
                    .negotiation_protocol
                    .send_response(channel, NegotiationResponse::SolutionAck);
            }
        }
    }

    /// Response handler for [NegotiationResponse]. See [NegotiationResponse] to learn about
    /// the signifigance of each enum variant.
    async fn handle_negotiation_response(
        &mut self,
        _request_id: OutboundRequestId,
        response: NegotiationResponse,
    ) {
        match response {
            NegotiationResponse::AcknowledgementReceived(
                _from,
                negotiation_id,
                worker_signature,
                worker_address,
            ) => {
                // If we face a critical error here we need to cache this response and
                let shared_state = self.shared_state.lock().await;

                let negotiation_or_none =
                    shared_state.storage_manager.get_proposal(&negotiation_id);

                if negotiation_or_none.is_none() {
                    return;
                }

                let mut negotiation = negotiation_or_none.unwrap();
                negotiation.worker_signature = Some(worker_signature.clone());
                negotiation.status = Some(ProposalStatus::FullySigned {
                    employer_signature: negotiation.employer_signature.unwrap(),
                    worker_signature: worker_signature.clone(),
                });
                negotiation.worker_address = Some(worker_address.into());

                let _ = shared_state
                    .storage_manager
                    .save_or_update_proposal(&negotiation.clone())
                    .await;
            }
            NegotiationResponse::ProposalAcknowledgement(_from, negotiation_id) => {
                let negotiation_or_none = self
                    .shared_state
                    .lock()
                    .await
                    .storage_manager
                    .get_proposal(&negotiation_id);

                if negotiation_or_none.is_none() {
                    return;
                }

                let mut negotiation = negotiation_or_none.unwrap();
                negotiation.status = Some(ProposalStatus::Acknowledged);

                let _ = self
                    .shared_state
                    .lock()
                    .await
                    .storage_manager
                    .save_or_update_proposal(&negotiation)
                    .await;
            }
            NegotiationResponse::ActiveWorkAcknowledgement(work) => {
                // If creation of the chain transaction fails here the transaction should be cached alongside the work item
                // to be resubmitted to the chain. The on chain time lock algorithm will track the creation date/time of
                // the work so we can submit this transaction when available and the working node will eventually discard
                // the negotiation if too much time passes before a creation transaction occurs.
                let _ = self
                    .shared_state
                    .lock()
                    .await
                    .chain_manager
                    .create_work(&work)
                    .await;
            }
            NegotiationResponse::SolutionAck => {
                // We don't need to do anything here.
            }
        }
    }
}

impl GossipsubNodeProvider for CoNodeNetwork {
    /// Handles gossipsub messages that are published throughout the network.
    fn handle_gossipsub_message(
        &mut self,
        message: GossipsubMessage,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + Send {
        async move {
            // For any Work item published in the work discovery topic the network will verify the item meets the
            // nodes work criteria, store the peer for potential future interaction and store the broadcast in a database.

            // @dev Future work includes adding a setting to the network configurator allowed the client to accept a max number of
            // broadcast, i.e. opportunities
            if message.topic == work_discovery_topic().into() {
                if let Ok(work) = bincode::deserialize::<WorkBroadcast>(&message.data) {
                    log_info(format!(
                        "Discovered potential work opportunity from peer {}",
                        work.peer_info.peer_id.to_string()
                    ))
                    .await;
                    let _ = self.handle_potential_work(&work, &message).await;
                } else {
                    error!("Failed to deserialize work broadcast.");
                }
            }

            Ok(())
        }
    }

    /// Subscribes a node to the work discovery topic.
    fn join_gossip_protocol(
        &mut self,
    ) -> impl Future<Output = Result<bool, libp2p::gossipsub::SubscriptionError>> {
        async move {
            match self
                .swarm
                .behaviour_mut()
                .conode
                .gossipsub
                .subscribe(&work_discovery_topic())
            {
                Ok(success) => {
                    if success {
                        log_info("Successfully subscribed to topic: work_discovery".to_string())
                            .await;
                    } else {
                        log_warning("Failed to subscribe to topic: work_discovery".to_string())
                            .await;
                    }

                    Ok(success)
                }
                Err(err) => {
                    let cloned_err = err.borrow();
                    match cloned_err {
                        SubscriptionError::NotAllowed => {
                            log_warning(format!(
                                "[SubscriptionError]: Failed to subscribe to topic [work_discovery]: unauthorized"
                            ))
                            .await;
                        }
                        SubscriptionError::PublishError(e) => {
                            log_warning(format!(
                                "[PublishError]: Failed to subscribe to topic [work discovery]: {}",
                                e.to_string()
                            ))
                            .await;
                        }
                    }
                    Err(err)
                }
            }
        }
    }
}
