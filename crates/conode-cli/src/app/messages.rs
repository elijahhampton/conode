// src/app/messages.rs
use crate::types::enums::*;
use crate::ui::views::main::MainContentView;
use conode_logging::logger::LogLevel;
use conode_protocol::event::{NetworkEvent, NodeCommand};
use conode_protocol::labor_market_node::Node;

use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::sync::SyncEvent;
use conode_types::work::{ActiveWork, WorkBroadcast};
use iced_aw::date_picker::Date;
use libp2p::PeerId;
use starknet::core::types::Felt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::{mpsc, MutexGuard};


#[derive(Debug, Clone)]
pub enum Message {
    // Messages related to proposals
    InitiateNegotiation(WorkBroadcast),
    FilterMarket(String, bool),
    //
    OptionsView,
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
        Option<Arc<tokio::sync::Mutex<Node>>>,
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

    AcknowledgeProposal(Negotiation),
    SignProposal(Negotiation),
    SignAndConfirmProposal(Negotiation),
    CreateWorkOnChain(Negotiation),
    ProposalsFetched(Vec<Negotiation>),
    TaskDiscovered(Vec<WorkBroadcast>),
    OnSignProposalCallback,
    SolutionChanged(String, String), // (work_id, solution)
    SubmitSolution(ActiveWork),
    ReviewSolution(ActiveWork),
    ActiveWorksFetched((Vec<ActiveWork>, HashMap<String, Option<Felt>>)),
    SeedWordChanged(usize, String), // tracking input changes
    RestoreAndStartNode(Vec<String>),
    SolutionDecision,
    SolutionTestedInternally(String, Felt),

    // Syncing
    SyncEvent(SyncEvent),
    UpdateSyncStatus,
    InitSyncReceiver(tokio::sync::watch::Receiver<SyncEvent>),
    TabChanged(String),
    SwitchMainView(MainContentView),

    // iced_aw DatePicker
    ChooseDate,
    SubmitDate(Date),
    CancelDate,
}

unsafe impl Send for Message {}
unsafe impl Sync for Message {}

impl Default for Message {
    fn default() -> Self {
        Message::Noop
    }
}
