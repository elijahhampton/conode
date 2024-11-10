// src/app/messages.rs
use crate::types::enums::*;
use conode_logging::logger::LogLevel;
use conode_protocol::event::{NetworkEvent, NodeCommand};
use conode_protocol::labor_market_node::LaborMarketNode;
use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::work::{ActiveWork, WorkBroadcast};
use libp2p::PeerId;
use starknet::core::types::Felt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;

#[derive(Clone, Debug)]
pub enum Message {
    CreateAccount,
    RestoreFromSeed,
    StartNode,
    NavigateTo(View),
    ViewLogs,
    ViewOpportunities,
    ViewProposals,
    ViewActiveWork,
    ViewCompletedWork,
    BroadcastWork,
    AddLog(String, LogLevel),
    BatchLog(Vec<Message>),
    NodeInitialized(
        Option<LaborMarketNode>,
        Option<mpsc::Sender<NodeCommand>>,
        Option<Arc<TokioMutex<mpsc::Receiver<NetworkEvent>>>>,
        Option<PeerId>,
        Option<PeerInfo>,
    ),
    NodeInitializationFailed(String),
    NetworkEvent(NetworkEvent),
    Noop,
    BroadcastFormSubmit,
    ExpiryDateSelected(ExpiryOption),
    RewardChanged(String),
    RequirementsChanged(String),
    DescriptionChanged(String),
    SwitchTab(WorkTab),
    SelectWorkItem(String),
    WorkItemsFetched(Vec<WorkBroadcast>),
    InitiateNegotiation(WorkBroadcast),
    AcknowledgeProposal(Negotiation),
    SignProposal(Negotiation),
    SignAndConfirmProposal(Negotiation),
    CreateWorkOnChain(Negotiation),
    ProposalsFetched(Vec<Negotiation>),
    OnSignProposalCallback,
    SolutionChanged(String, String), // (work_id, solution)
    SubmitSolution(ActiveWork),
    ReviewSolution(ActiveWork),
    ActiveWorksFetched((Vec<ActiveWork>, HashMap<String, Option<Felt>>)),
    SeedWordChanged(usize, String), // tracking input changes
    RestoreAndStartNode(Vec<String>),
    SolutionDecision,
    SolutionTestedInternally(String, Felt),
}
