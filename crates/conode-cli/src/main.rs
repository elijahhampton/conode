#[warn(clippy::all)]
#[allow(clippy::all)]
mod app;
mod network;
mod types;
mod ui;

use ::libp2p::PeerId;
use app::messages::Message;
use app::state::BroadcastFormState;
use conode_logging::logger::{initialize_logger, log_warning, AsyncLogger, LogLevel};
use conode_protocol::event::{NetworkEvent, NodeCommand};
use conode_protocol::labor_market_node::LaborMarketNode;
use conode_starknet::crypto::keypair::KeyPair;
use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::work::{ActiveWork, Work, WorkBroadcast};

use iced::{executor, theme::Theme, Application, Command, Element, Settings, Subscription};
use log::{info, warn};
use network::event_stream::EventStream;
use starknet::core::types::Felt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as TimeDuration;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use types::enums::{View, WorkTab};
use ui::views::active_work::ActiveWorkView;
use ui::views::logs::LogsView;
use ui::views::proposals::ProposalView;

use crate::ui::views::broadcast::BroadcastView;
use crate::ui::views::main::MainView;
use crate::ui::views::mnemonic::MnemonicView;
use crate::ui::views::options::OptionsView;
use crate::ui::views::restore_seed::RestoreSeedView;
use crate::ui::views::work::WorkView;

pub struct ConodeCLI {
    current_view: View,
    mnemonic_words: Vec<String>,
    node: Option<LaborMarketNode>,
    command_tx: Option<mpsc::Sender<NodeCommand>>,
    event_rx: Option<Arc<Mutex<mpsc::Receiver<NetworkEvent>>>>,
    node_running: bool,
    node_runner: Option<tokio::task::JoinHandle<()>>,
    logger: Arc<Mutex<Option<AsyncLogger>>>,
    broadcast_form: BroadcastFormState,
    current_work_tab: WorkTab,
    work_items: Vec<WorkBroadcast>,
    local_peer_id: Option<PeerId>,
    local_peer_info: Option<PeerInfo>,
    proposals: Vec<Negotiation>,
    active_works: Vec<ActiveWork>,
    recover_seed_words: Vec<String>,
    active_work_solutions: HashMap<String, String>,
    active_work_solutions_submitted: HashMap<String, Option<Felt>>,
}

impl Application for ConodeCLI {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Arc<TokioMutex<Option<AsyncLogger>>>;

    fn new(logger: Arc<TokioMutex<Option<AsyncLogger>>>) -> (Self, Command<Message>) {
        let new_wallet_keypair = KeyPair::generate();
        let mnemonic_words = new_wallet_keypair
            .mnemonic()
            .unwrap()
            .clone()
            .into_phrase()
            .split_whitespace()
            .map(String::from)
            .collect();

        let future = async move {
            match LaborMarketNode::new().await {
                Ok((node, command_tx, event_rx, peer_id, peer_info)) => Message::NodeInitialized(
                    Some(node),
                    Some(command_tx),
                    Some(Arc::new(TokioMutex::new(event_rx))),
                    Some(peer_id),
                    Some(peer_info),
                ),
                Err(e) => Message::NodeInitializationFailed(e.to_string()),
            }
        };

        let initial_command = Command::perform(future, std::convert::identity);

        (
            Self {
                current_view: View::Main,
                mnemonic_words,
                node: None,
                node_runner: None,
                command_tx: None,
                event_rx: None,
                node_running: false,
                logger,
                broadcast_form: BroadcastFormState {
                    expiry_date: None,
                    reward: String::new(),
                    requirements: String::new(),
                    description: String::new(),
                },
                current_work_tab: WorkTab::Stored,
                work_items: Vec::new(),
                local_peer_id: None,
                local_peer_info: None,
                proposals: Vec::new(),
                active_works: Vec::new(),
                recover_seed_words: vec![String::new(); 24],
                active_work_solutions: HashMap::new(),
                active_work_solutions_submitted: HashMap::new(),
            },
            initial_command,
        )
    }

    fn title(&self) -> String {
        String::from("Conode Dashboard")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NodeInitialized(node, command_tx, event_rx, peer_id, peer_info) => {
                self.node = node;
                self.command_tx = command_tx;
                self.event_rx = event_rx;
                self.local_peer_id = peer_id;
                self.local_peer_info = peer_info;
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger
                                .log(LogLevel::Info, "Node initialized successfully".to_string())
                                .await;
                        }
                    },
                    // async move {
                    //     if let Ok(logger_guard) = logger.lock().await {
                    //         if let Some(logger) = logger_guard.as_ref() {
                    //             logger.log(LogLevel::Info, "Node initialized successfully".to_string()).await;
                    //         }
                    //     }
                    // },
                    |_| Message::Noop,
                )
            }
            Message::NodeInitializationFailed(error) => {
                let error_msg = format!("Node initialization failed: {}", error);
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger.log(LogLevel::Error, error_msg).await;
                        }
                    },
                    // async move {
                    //     if let Ok(logger_guard) = logger.lock().await {
                    //         if let Some(logger) = logger_guard.as_ref() {
                    //             logger.log(LogLevel::Error, error_msg).await;
                    //         }
                    //     }
                    // },
                    |_| Message::Noop,
                )
            }
            Message::CreateAccount => {
                self.current_view = View::Mnemonic;
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger
                                .log(LogLevel::Info, "Account creation initiated".to_string())
                                .await;
                        }
                    },
                    // async move {
                    //     if let Ok(logger_guard) = logger.lock().await {
                    //         if let Some(logger) = logger_guard.as_ref() {
                    //             logger.log(LogLevel::Info, "Account creation initiated".to_string()).await;
                    //         }
                    //     }
                    // },
                    |_| Message::Noop,
                )
            }
            Message::StartNode => {
                self.current_view = View::Options;
                if self.node.is_none() {
                    warn!("Warning: Node is None when trying to start");
                }
                self.start_node()
            }
            Message::NavigateTo(view) => {
                self.current_view = view;
                Command::none()
            }
            Message::ViewLogs => {
                self.current_view = View::Logs;
                Command::none()
            }
            Message::ViewOpportunities => {
                self.current_view = View::WorkOpportunities;
                self.fetch_work_items() // Fetch items when view is opened
            }
            Message::WorkItemsFetched(items) => {
                self.work_items = items;
                Command::none()
            }
            Message::SwitchTab(tab) => {
                self.current_work_tab = tab.clone();
                if matches!(tab.clone(), WorkTab::Saved) {
                    self.fetch_work_items()
                } else {
                    Command::none()
                }
            }
            Message::BroadcastWork => {
                self.current_view = View::BroadcastForm;
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger
                                .log(LogLevel::Info, "Broadcast Work clicked".to_string())
                                .await;
                        }
                    },
                    |_| Message::Noop,
                )
            }
            Message::AddLog(log, level) => {
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger.log(level, log).await;
                        }
                    },
                    |_| Message::Noop,
                )
            }
            Message::BatchLog(messages) => {
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            for message in messages {
                                if let Message::AddLog(log, level) = message {
                                    logger.log(level, log).await;
                                }
                            }
                        }
                    },
                    |_| Message::Noop,
                )
            }
            Message::NetworkEvent(event) => {
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger
                                .log(
                                    LogLevel::Info,
                                    format!("Network event received: {:?}", event),
                                )
                                .await;
                        }
                    },
                    // async move {
                    //     if let Ok(logger_guard) = logger.lock().await {
                    //         if let Some(logger) = logger_guard.as_ref() {
                    //             logger.log(LogLevel::Info, format!("Network event received: {:?}", event)).await;
                    //         }
                    //     }
                    // },
                    |_| Message::Noop,
                )
            }
            Message::Noop => Command::none(),
            Message::ExpiryDateSelected(expiry) => {
                self.broadcast_form.expiry_date = Some(expiry);
                Command::none()
            }
            Message::RewardChanged(value) => {
                self.broadcast_form.reward = value;
                Command::none()
            }
            Message::RequirementsChanged(value) => {
                self.broadcast_form.requirements = value;
                Command::none()
            }
            Message::DescriptionChanged(value) => {
                self.broadcast_form.description = value;
                Command::none()
            }
            Message::BroadcastFormSubmit => {
                if let Some(_expiry) = self.broadcast_form.expiry_date {
                    let logger = Arc::clone(&self.logger);
                    let form_data = self.broadcast_form.clone();

                    // Split requirements
                    let requirements: Vec<String> = form_data
                        .requirements
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    // Clone what we need
                    let node = self.node.clone();
                    let command_tx = self.command_tx.clone();

                    Command::perform(
                        async move {
                            if let (Some(node), Some(command_tx)) = (node, command_tx) {
                                let peer_info = node.peer_info().await;

                                let work = Work::new(
                                    form_data.description,
                                    requirements,
                                    None,
                                    Some(form_data.reward.parse().unwrap_or(0)),
                                    None,
                                    Some(node.starknet_address().await),
                                    None,
                                );

                                let work_broadcast = WorkBroadcast::new(work, peer_info);

                                match command_tx
                                    .send(NodeCommand::BroadcastJob(work_broadcast))
                                    .await
                                {
                                    Ok(_) => {
                                        let logger_guard = logger.lock().await;
                                        if let Some(logger) = logger_guard.as_ref() {
                                            logger
                                                .log(
                                                    LogLevel::Info,
                                                    "Work broadcast successful".to_string(),
                                                )
                                                .await;
                                        }
                                        true
                                    }
                                    Err(e) => {
                                        let logger_guard = logger.lock().await;
                                        if let Some(logger) = logger_guard.as_ref() {
                                            logger
                                                .log(
                                                    LogLevel::Error,
                                                    format!("Failed to broadcast work: {}", e),
                                                )
                                                .await;
                                        }
                                        false
                                    }
                                }
                            } else {
                                let logger_guard = logger.lock().await;
                                if let Some(logger) = logger_guard.as_ref() {
                                    logger
                                        .log(LogLevel::Error, "Node not initialized".to_string())
                                        .await;
                                }
                                false
                            }
                        },
                        |success| {
                            if success {
                                Message::NavigateTo(View::Options)
                            } else {
                                Message::Noop
                            }
                        },
                    )
                } else {
                    let logger = Arc::clone(&self.logger);
                    Command::perform(
                        async move {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Error,
                                        "Cannot broadcast work: Expiry date not selected"
                                            .to_string(),
                                    )
                                    .await;
                            }
                        },
                        |_| Message::Noop,
                    )
                }
            }

            Message::SelectWorkItem(work_id) => {
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger
                                .log(LogLevel::Info, format!("Selected work item: {}", work_id))
                                .await;
                        }
                    },
                    |_| Message::Noop,
                )
            }
            Message::InitiateNegotiation(broadcast) => {
                let node = self.node.clone().unwrap();

                let recipient = broadcast.peer_info.peer_id;
                let job_id = broadcast.work.id;
                let reward = broadcast.work.details.reward;
                let employer = broadcast.peer_info.peer_id;
                let peer_info = broadcast.peer_info.clone();
                let negotiation = Negotiation::new(
                    Some(peer_info),
                    self.local_peer_info.clone(),
                    employer,
                    job_id.clone(),
                    reward,
                );

                Command::perform(
                    async move {
                        node.network
                            .initiate_negotiation(recipient, job_id, negotiation)
                            .await;
                    },
                    |_| Message::Noop,
                )
            }
            Message::ViewProposals => {
                self.current_view = View::Proposals;
                self.fetch_proposals()
            }
            Message::ProposalsFetched(proposals) => {
                self.proposals = proposals;
                Command::none()
            }
            Message::ViewCompletedWork => todo!(),
            Message::AcknowledgeProposal(_) => todo!(),
            Message::SignProposal(negotiation) => {
                let negotiation_clone = negotiation.clone();
                let worker_peer_info = negotiation.worker_peer_info.unwrap().peer_id;
                let node = self.node.clone().unwrap();

                Command::perform(
                    async move {
                        node.network
                            .request_completion_ack(worker_peer_info, negotiation_clone)
                            .await;
                    },
                    |_| Message::OnSignProposalCallback,
                )
            }
            Message::OnSignProposalCallback => {
                let _ = self.fetch_proposals();
                Command::none()
            }
            Message::SignAndConfirmProposal(_) => todo!(),
            Message::CreateWorkOnChain(proposal) => {
                info!("Creating work on chain.. Calling and sending emssage from cli");
                let node = self.node.clone().unwrap();
                Command::perform(
                    async move {
                        node.network
                            .send_completion_confirmation_request(
                                proposal.worker_peer_info.unwrap().peer_id,
                                proposal.id,
                            )
                            .await;
                    },
                    |_| Message::Noop,
                )
            }
            Message::ViewActiveWork => {
                self.current_view = View::ActiveWork;
                self.fetch_active_works()
            }
            Message::ActiveWorksFetched(works) => {
                self.active_works = works.0;
                self.active_work_solutions_submitted = works.1;

                for (_work_id, solution_opt) in &self.active_work_solutions_submitted {
                    match solution_opt {
                        Some(_solution) => {}
                        None => {}
                    }
                }

                Command::none()
            }
            Message::ReviewSolution(_active_work) => {
                // TODO: Implement solution review logic
                Command::none()
            }
            Message::SeedWordChanged(index, value) => {
                if index < self.recover_seed_words.len() {
                    self.recover_seed_words[index] = value;
                }
                Command::none()
            }
            Message::RestoreAndStartNode(_) => {
                // Validate seed words
                info!("Restore and start node....");
                if self.recover_seed_words.iter().all(|word| !word.is_empty()) {
                    // Create keypair from seed words
                    match KeyPair::from_mnemonic(&self.recover_seed_words.join(" ")) {
                        Ok(_) => {
                            info!("Starting node");
                            self.current_view = View::Options;
                            // Continue with normal node startup
                            self.start_node()
                        }
                        Err(_) => {
                            info!("error");
                            // Handle invalid seed phrase
                            Command::none()
                        }
                    }
                } else {
                    info!("incomplete seed phrase");
                    // Handle incomplete seed phrase
                    Command::none()
                }
            }
            Message::RestoreFromSeed => {
                self.current_view = View::RestoreSeed;
                Command::none()
            }
            Message::SolutionChanged(work_id, solution) => {
                self.active_work_solutions.insert(work_id, solution);
                Command::none()
            }
            Message::SolutionTestedInternally(work_id, solution) => {
                let node = match self.node.clone() {
                    Some(node) => node,
                    None => return Command::none(),
                };

                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                // Uses Iced's Command::perform properly
                Command::perform(
                    async move {
                        runtime.block_on(async {
                            node.verify_and_complete_solution(work_id, solution).await
                        })
                    },
                    |_| Message::SolutionDecision,
                )
            }
            Message::SubmitSolution(active_work) => {
                if let Some(solution) = self.active_work_solutions.get(&active_work.work.id) {
                    let node = self.node.clone();
                    let work_id = active_work.work.id.clone();
                    let solution = solution.clone();

                    Command::perform(
                        async move {
                            if let Some(mut node) = node {
                                let active_work = node.get_active_work_by_id(work_id).await;

                                match active_work {
                                    Ok(work) => {
                                        if let Some(work) = work {
                                            info!(
                                                "Submitting completion confirmation ack {}",
                                                work.employer_peer_id.clone().to_string()
                                            );

                                            node.network.send_completion_confirmation_ack(
                                                &work.employer_peer_id,
                                                work.work.id,
                                                solution,
                                            );
                                        }
                                    }
                                    Err(_) => {
                                        log_warning(format!("Unable send the completion confirmation acknowledgment request.")).await;
                                    }
                                }
                            }
                        },
                        |_| Message::Noop,
                    )
                } else {
                    Command::none()
                }
            }
            Message::SolutionDecision => {
                info!("Solution Accepted");
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match self.current_view {
            View::Main => self.main_view(),
            View::Mnemonic => self.mnemonic_view(),
            View::Options => self.options_view(),
            View::Logs => self.logs_view(),
            View::BroadcastForm => self.broadcast_form_view(),
            View::WorkOpportunities => self.work_opportunities_view(),
            View::Proposals => self.proposals_view(),
            View::ActiveWork => self.active_work_view(),
            View::RestoreSeed => self.restore_seed_view(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Some(ref event_rx) = self.event_rx {
            iced::Subscription::from_recipe(EventStream {
                event_rx: Arc::clone(event_rx),
            })
        } else {
            Subscription::none()
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl ConodeCLI {
    #[allow(dead_code)]
    fn get_solution_input(&self, work_id: &str) -> String {
        self.active_work_solutions
            .get(work_id)
            .cloned()
            .unwrap_or_default()
    }

    fn fetch_active_works(&mut self) -> Command<Message> {
        let node = self.node.clone();
        let logger = Arc::clone(&self.logger);

        Command::perform(
            async move {
                if let Some(node) = node {
                    match node.list_active_work().await {
                        Ok(works) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Info,
                                        format!("Fetched {} active works", works.len()),
                                    )
                                    .await;
                            }

                            let solutions = node
                                .active_work_with_solutions(works.clone())
                                .await
                                .unwrap();

                            (works, solutions)
                        }
                        Err(e) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Error,
                                        format!("Failed to fetch active works: {}", e),
                                    )
                                    .await;
                            }
                            (Vec::new(), HashMap::new())
                        }
                    }
                } else {
                    (Vec::new(), HashMap::new())
                }
            },
            Message::ActiveWorksFetched,
        )
    }

    fn fetch_proposals(&mut self) -> Command<Message> {
        let node = self.node.clone();
        let logger = Arc::clone(&self.logger);

        Command::perform(
            async move {
                if let Some(node) = node {
                    match node.list_proposals().await {
                        Ok(proposals) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Info,
                                        format!("Fetched {} proposals", proposals.len()),
                                    )
                                    .await;
                            }
                            proposals
                        }
                        Err(e) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Error,
                                        format!("Failed to fetch proposals: {}", e),
                                    )
                                    .await;
                            }
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                }
            },
            Message::ProposalsFetched,
        )
    }

    fn start_node(&mut self) -> Command<Message> {
        if let Some(mut node) = self.node.clone() {
            let (_, rx) = mpsc::channel::<(String, LogLevel)>(100);
            let logger = Arc::clone(&self.logger);

            let runner = tokio::spawn(async move {
                if let Err(e) = node.run().await {
                    let logger_guard = logger.lock().await;
                    if let Some(logger) = logger_guard.as_ref() {
                        logger
                            .log(LogLevel::Error, format!("Node error: {:?}", e))
                            .await;
                    }
                } else {
                    let logger_guard = logger.lock().await;
                    if let Some(_logger) = logger_guard.as_ref() {
                        if let Some(logger) = logger_guard.as_ref() {
                            logger.log(LogLevel::Info, "Node stopped".to_string()).await;
                        }
                    }
                }
            });

            self.node_runner = Some(runner);

            self.node_running = true;

            Command::perform(
                async move {
                    ReceiverStream::new(rx)
                        .map(|(msg, level)| Message::AddLog(msg, level))
                        .collect::<Vec<_>>()
                        .await
                },
                Message::BatchLog,
            )
        } else {
            let logger = Arc::clone(&self.logger);

            Command::perform(
                async move {
                    let logger_guard = logger.lock().await;
                    if let Some(logger) = logger_guard.as_ref() {
                        logger
                            .log(
                                LogLevel::Error,
                                "Failed to start node: No node instance available".to_string(),
                            )
                            .await;
                    }
                },
                // async move {
                //     if let Ok(logger_guard) = logger.lock().await {
                //         if let Some(logger) = logger_guard.as_ref() {
                //             logger.log(
                //                 LogLevel::Error,
                //                 "Failed to start node: No node instance available".to_string()
                //             ).await;
                //         }
                //     }
                // },
                |_| Message::Noop,
            )
        }
    }

    fn fetch_work_items(&mut self) -> Command<Message> {
        let node = self.node.clone();
        let logger = Arc::clone(&self.logger);

        Command::perform(
            async move {
                if let Some(node) = node {
                    match node.list_work_opportunities().await {
                        Ok(items) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Info,
                                        format!("Fetched {} work items", items.len()),
                                    )
                                    .await;
                            }
                            items
                        }
                        Err(e) => {
                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Error,
                                        format!("Failed to fetch work items: {}", e),
                                    )
                                    .await;
                            }
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                }
            },
            |items| Message::WorkItemsFetched(items),
        )
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> iced::Result {
    let logger = initialize_logger("conode.log", TimeDuration::from_secs(5)).await;
    let logger_arc = Arc::new(TokioMutex::new(Some(logger)));

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .filter(Some("iced_wgpu"), log::LevelFilter::Error)
        .filter(Some("iced_winit"), log::LevelFilter::Error)
        .filter(Some("wgpu"), log::LevelFilter::Error)
        .init();

    let _settings = Settings::with_flags(Arc::clone(&logger_arc));
    let mut settings = Settings::with_flags(Arc::clone(&logger_arc));
    settings.window.size = (600, 600);
    settings.window.resizable = false;

    match ConodeCLI::run(settings) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
