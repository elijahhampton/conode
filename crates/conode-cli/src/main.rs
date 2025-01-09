#[warn(clippy::all)]
#[allow(clippy::all)]
mod app;
mod network;
mod types;
mod ui;

use iced::widget::{container, Button, Column, Container, Row, Text};
use iced_aw::date_picker::Date;
use ::libp2p::PeerId;
use app::messages::Message;
use app::state::BroadcastFormState;
use conode_logging::logger::{initialize_logger, log_warning, AsyncLogger, LogLevel};
use conode_protocol::event::{NetworkEvent, NodeCommand};
use conode_protocol::labor_market_node::{Node};
use conode_starknet::crypto::keypair::KeyPair;
use conode_types::negotiation::Negotiation;
use conode_types::peer::PeerInfo;
use conode_types::sync::SyncEvent;
use conode_types::work::{ActiveWork, Work, WorkBroadcast};
use iced::{subscription, theme, time, Length};
use iced::window::Position;
use network::event_stream::EventStream;

use iced::{executor, theme::Theme, Application, Command, Element, Settings};
use log::info;
//use network::event_stream::EventStream;
use starknet::core::types::Felt;
use ui::func::gui::traits::create::CreateComponent;
use ui::state::buttons::ToolbarButtonsState;
use ui::styles::component::{PaymentBadgeStyle, SyncStatusStyle};
use ui::styles::container::OuterContainerStyle;
use ui::views::main::MainContentView;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as TimeDuration;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use types::enums::{View, WorkTab};
use crate::ui::views::main::MainView;
use crate::ui::styles::component::TaskItemStyle;
use crate::ui::styles::button::ModernButtonStyle;

/// The state of the iced GUI.
pub struct GUIState {
    /// Interface state
    // The current content view displayed.
    current_main_view: MainContentView,
    // State for toolbar buttons.
    toolbar_buttons_state: ToolbarButtonsState,

    /// Network state
    // Broadcasted task received through Gossipsub
    validated_task_broadcast: Vec<WorkBroadcast>,
    // Params for the expiry date DatePicker

    /// The mnemonic created for new wallets
    mnemonic_words: Vec<String>,
    /// The recovered mnemonic for existing wallets
    recover_seed_words: Vec<String>,
    /// The underlying [`Node`]
    node: Option<Arc<Mutex<Node>>>,

    /// Node running state
    node_running: bool,
    /// A JoinHandle containing the underlying node run() task
    node_runner: Option<tokio::task::JoinHandle<()>>,
    /// A asynchronous logger used for displaying GUI events
    logger: Arc<Mutex<Option<AsyncLogger>>>,
    /// [`PeerId`] for the underlying [`Node`]
    local_peer_id: Option<PeerId>,
    /// [`PeerInfo`] for the underlying [`Node`]
    local_peer_info: Option<PeerInfo>,
    /// State of the broadcast form
    broadcast_form: BroadcastFormState,
    /// Current displayed work tab
    current_work_tab: WorkTab,

    /// Received Proposals
    proposals: Vec<Negotiation>,
    /// Active task
    active_works: Vec<ActiveWork>,
    /// Active task solutions
    active_work_solutions: HashMap<String, String>,
    /// Active task solutions submitted
    active_work_solutions_submitted: HashMap<String, Option<Felt>>,
    sync_status: Option<SyncEvent>,
    sync_progress_message: String,
    sync_event_receiver: Option<tokio::sync::watch::Receiver<SyncEvent>>,

}

impl Application for GUIState {
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
            match Node::new().await {
                Ok((node, peer_id, peer_info)) => {
                    Message::NodeInitialized(
                        Some(Arc::new(Mutex::new(node))),
                        Some(peer_id),
                        Some(peer_info),
                    )
                }
                Err(e) => Message::NodeInitializationFailed(e.to_string()),
            }
        };

        let initial_command = Command::perform(future, |msg| msg);

        (
            Self {
                current_main_view: MainContentView::ViewTasks,
                mnemonic_words,
                node: None,
                node_runner: None,

                node_running: false,
                logger,
                broadcast_form: BroadcastFormState {
                    reward: String::new(),
                    requirements: String::new(),
                    description: String::new()
                },
                current_work_tab: WorkTab::Stored,
                validated_task_broadcast: Vec::new(),
                local_peer_id: None,
                local_peer_info: None,
                proposals: Vec::new(),
                active_works: Vec::new(),
                recover_seed_words: vec![String::new(); 24],
                active_work_solutions: HashMap::new(),
                active_work_solutions_submitted: HashMap::new(),
                sync_progress_message: "Node initializing...".to_string(),
                sync_event_receiver: None,
                sync_status: None,
                toolbar_buttons_state: ToolbarButtonsState::default()
            },
            initial_command,
        )
    }

    fn title(&self) -> String {
        String::from("Conode Dashboard")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        match (&self.node, &self.sync_event_receiver) {
            (Some(_), Some(receiver)) => iced::Subscription::batch(vec![
                subscription::run_with_id(
                    "sync_events",
                    Box::new(EventStream::new(receiver.clone())),
                ),
                time::every(tokio::time::Duration::from_secs(1)).map(|_| Message::UpdateSyncStatus),
            ]),
            _ => iced::Subscription::none(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchMainView(view) => {
                self.current_main_view = view.clone();
                match view {
                    MainContentView::DiscoverTask => {
                        let _ = self.fetch_discovered_task();
                    }
                    MainContentView::ViewTasks => todo!(),
                    MainContentView::PublishTask => {}
                    MainContentView::ViewDetails => todo!(),
                    MainContentView::ViewProposals => todo!(),
                }
                Command::none()
            }
            // Proposal related [`Message`]s
            Message::InitiateNegotiation(broadcast) => {
                let node = self.node.clone().unwrap();

                let recipient = broadcast.peer_info.peer_id;
                let job_id = broadcast.work.id;
                let reward = broadcast.work.details.reward;
                let employer = broadcast.peer_info.peer_id;
                let peer_info = broadcast.peer_info.clone();
                let mut negotiation = Negotiation::new(
                    Some(peer_info),
                    self.local_peer_info.clone(),
                    employer,
                    job_id.clone(),
                    reward,
                );

                Command::perform(
                    async move {
                        node.lock()
                            .await
                            .network
                            
                            .initiate_negotiation(recipient, job_id, &mut negotiation)
                            .await;
                    },
                    |_| Message::Noop,
                )
            }
            Message::ViewProposals => {
                self.fetch_proposals()
            }
            Message::TaskDiscovered(task) => {
                self.validated_task_broadcast = task;
                Command::none()
            }
            Message::ProposalsFetched(proposals) => {
                self.proposals = proposals;
                Command::none()
            }
            Message::InitSyncReceiver(receiver) => {
                self.sync_event_receiver = Some(receiver);
                Command::perform(async move {}, |_| Message::Noop)
            }
            Message::OptionsView => {
               
                Command::none()
            }
            Message::SyncEvent(event) => {
                self.sync_status = Some(event);
                // let noop_handle_command = self.handle_sync_event(event).await;
                let noop_update_command = self.update_sync_ui();

                Command::perform(async move { Message::Noop }, |_| Message::Noop)
            }
            Message::NodeInitialized(node, peer_id, peer_info) => {
                self.node = node;
                self.local_peer_id = peer_id;
                self.local_peer_info = peer_info;

                // Store the sync event receiver now, outside the async block
                if let Some(node) = &self.node {
                    let node = Arc::clone(node);
                    let logger = Arc::clone(&self.logger);
                    
                    Command::perform(
                        async move {
                            // Get the receiver inside the async block
                            let receiver = node.lock().await.state_service.subscribe_to_events();

                            let logger_guard = logger.lock().await;
                            if let Some(logger) = logger_guard.as_ref() {
                                logger
                                    .log(
                                        LogLevel::Info,
                                        "Node initialized successfully".to_string(),
                                    )
                                    .await;
                            }

                            // Return the receiver to be handled in the callback
                            receiver
                        },
                        |receiver| Message::InitSyncReceiver(receiver),
                    )
                } else {
                    Command::none()
                }
            }
            Message::NodeInitializationFailed(error) => {
                info!("{}", error.to_string());
                let error_msg = format!("Node initialization failed: {}", error);
                let logger = Arc::clone(&self.logger);
                Command::perform(
                    async move {
                        let logger_guard = logger.lock().await;
                        if let Some(logger) = logger_guard.as_ref() {
                            logger.log(LogLevel::Error, error_msg).await;
                        }
                    },
                    |_| Message::Noop,
                )
            }
            Message::StartNode => {
                self.start_node();
                Command::perform(async move {}, |_| Message::Noop)
            }
            Message::CreateAccount => {
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
                    |_| Message::Noop,
                )
            }

            Message::NavigateTo(_) => {
              //  self.current_view = view;
                Command::none()
            }
            Message::ViewLogs => {
              
                Command::none()
            }
            Message::ViewOpportunities => {
                self.fetch_work_items();
                info!("FTECHING WOKR ITEMS");
                Command::none()
           
            }
            Message::WorkItemsFetched(items) => {
                self.validated_task_broadcast = items;
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
                    |_| Message::Noop,
                )
            }
            Message::Noop => Command::none(),
            Message::ExpiryDateSelected(expiry) => {
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
                if true {
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

                    Command::perform(
                        async move {
                            if let (Some(node)) = (node) {
                                let node_lock = node.lock().await;
                                let peer_info = node_lock.network.peer_info().await.unwrap();

                                let work = Work::new(
                                    form_data.description,
                                    requirements,
                                    None,
                                    Some(form_data.reward.parse().unwrap_or(0)),
                                    None,
                                    Some(node_lock.starknet_address().await),
                                    None,
                                );

                                let work_broadcast = WorkBroadcast::new(work, peer_info);

                                false
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

            Message::ViewCompletedWork => todo!(),
            Message::AcknowledgeProposal(_) => todo!(),
            Message::SignProposal(negotiation) => {
                let negotiation_clone = negotiation.clone();
                let worker_peer_info = negotiation.worker_peer_info.unwrap().peer_id;
                let node = self.node.clone().unwrap();

                Command::perform(
                    async move {
                        let node_lock = node.lock().await;
                        node_lock
                            .network
                        
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
                        node.lock()
                            .await
                            .network
                   
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
                if self.recover_seed_words.iter().all(|word| !word.is_empty()) {
                    match KeyPair::from_mnemonic(&self.recover_seed_words.join(" ")) {
                        Ok(_) => {
                            self.start_node();
                            Command::none()
                        }
                        Err(_) => {
                            info!("error");
                            Command::none()
                        }
                    }
                } else {
                    info!("incomplete seed phrase");
                    Command::none()
                }
            }
            Message::RestoreFromSeed => {
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

                Command::perform(
                    async move {
                        node.lock()
                            .await
                            .verify_and_complete_solution(work_id, solution)
                            .await
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
                            if let Some(node) = node {
                                let  node_lock = node.lock().await;
                                let active_work = node_lock.get_active_work_by_id(work_id).await;

                                match active_work {
                                    Ok(work) => {
                                        if let Some(work) = work {
                                            info!(
                                                "Submitting completion confirmation ack {}",
                                                work.employer_peer_id.clone().to_string()
                                            );

                                            node_lock
                                                .network
                                        
                                                .send_completion_confirmation_ack(
                                                    &work.employer_peer_id,
                                                    work.work.id,
                                                    solution,
                                                ).await;
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
                Command::none()
            }
            Message::UpdateSyncStatus => {
                let _ = self.update_sync_ui();
                Command::none()
            }
            Message::TabChanged(_) => todo!(),
            Message::FilterMarket(_, _) => todo!(),
            Message::ChooseDate => todo!(),
            Message::SubmitDate(_) => todo!(),
            Message::CancelDate => todo!(),
        }
    }
    fn view(&self) -> Element<Message> {
        let content = self.main_view();
    
        // Create the sync status bar
        let sync_status = Container::new(
            Row::new()
                .push(Text::new(&self.sync_progress_message).size(12))
                .width(Length::Fill)
        )
        .width(Length::Fill)
        .padding(10)
        .style(theme::Container::Custom(Box::new(SyncStatusStyle)));
    
        Column::new()
            .push(sync_status)
            .push(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl CreateComponent for GUIState {
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Container::new(Text::new(label).size(12)) 
                .width(Length::Fill)
                .height(Length::Fixed(16.0)) 
                .center_y()
                .center_x(),
        )
        .width(Length::Shrink)
        .padding([2, 8]) 
        .style(theme::Button::Custom(Box::new(ModernButtonStyle)))
        .on_press(message)
     }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(1600.0))
            .height(Length::Fixed(900.0))
            .style(theme::Container::Custom(Box::new(OuterContainerStyle)))
            .center_x()
            .center_y()
            .into()
    }
    
    /// Creates a task item 
    fn create_task_item<'a>(
        &self,
        title: &'a str,
        payment: &'a str,
        requirements: &'a str,
        time: &'a str,
    ) -> Container<'a, Message> {
        Container::new(
            Row::new()
                .spacing(8)
                .push(
                    Column::new()
                        .width(Length::Fill)
                        .spacing(4)
                        .push(
                            Row::new()
                                .spacing(8)
                                .align_items(iced::Alignment::Start)
                                .push(
                                    Text::new(title)
                                        .size(13)
                                        .width(Length::Fill)
                                
                                )
                                .push(
                                    Container::new(Text::new(payment).size(12))
                                        .style(theme::Container::Custom(Box::new(PaymentBadgeStyle)))
                                        .padding([2, 6])
                                )
                        )
                        .push(
                            Row::new()
                                .spacing(8)
                                .push(
                                    Text::new(requirements)
                                        .size(11)
                                        .style(theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.7)))
                                )
                                .push(
                                    Text::new("•")
                                        .size(11)
                                        .style(theme::Text::Color(iced::Color::from_rgb(0.4, 0.4, 0.5)))
                                )
                                .push(
                                    Text::new(time)
                                        .size(11)
                                        .style(theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.6)))
                                )
                        )
                )
        )
        .style(theme::Container::Custom(Box::new(TaskItemStyle)))
        .padding([8, 12])
    }
}

impl GUIState {
    // fn handle_sync_event(&mut self, event: SyncEvent) -> Command<Message> {
    //     match event {
    //         SyncEvent::SyncStarted { from, to } => {
    //             self.sync_status = Some(SyncEvent::SyncProgress {
    //                 current: from,
    //                 target: to,
    //                 percentage: 0.0,
    //             });
    //         }
    //         SyncEvent::SyncProgress {
    //             current,
    //             target,
    //             percentage,
    //         } => {
    //             self.sync_status = Some(SyncEvent::SyncProgress {
    //                 current: current,
    //                 target: target,
    //                 percentage,
    //             });
    //         }
    //         SyncEvent::SyncCompleted => {
    //             self.sync_status = Some(SyncEvent::SyncCompleted);

    //             // When sync completes, refresh relevant UI data
    //             return Command::batch(vec![
    //                 self.fetch_work_items(),
    //                 self.fetch_proposals(),
    //                 self.fetch_active_works(),
    //             ]);
    //         }
    //         SyncEvent::SyncFailed => {
    //             self.sync_status = Some(SyncEvent::SyncFailed);
    //         }
    //     }
    //     Command::none()
    // }

    fn update_sync_ui(&mut self) -> Command<Message> {
        match &self.sync_status {
            Some(SyncEvent::SyncStarted { from, to }) => {
                self.sync_progress_message =
                    format!("Starting sync at block {} with target {}", from, to);
            }
            Some(SyncEvent::SyncProgress { percentage, .. }) => {
                self.sync_progress_message = format!("Syncing: {:.1}%", percentage);
            }
            Some(SyncEvent::SyncCompleted) => {
                self.sync_progress_message = "Up to date".to_string();
            }
            Some(SyncEvent::SyncFailed) => {
                self.sync_progress_message = "Sync failed.".to_string();
            }
            Some(SyncEvent::SyncAhead) => {
                self.sync_progress_message = "Waiting for initial sync.".to_string();
            }
            None => {
                self.sync_progress_message = if self.node_running {
                    "Waiting for initial sync."
                } else {
                    "Node is disconnected."
                }.to_string();
            }
        }
        Command::none()
    }

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
                    let node_lock = node.lock().await;
                    match node_lock.list_active_work().await {
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

                            let solutions = node_lock
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

    fn fetch_discovered_task(&self) -> Command<Message> {
        let node = self.node.clone();
        // let logger = Arc::clone(&self.logger);

        Command::perform(async move {
            if let Some(node) = node {
                let task = node.lock().await.list_task().await;
                task
            } else {
                Vec::new()
            }
        }, Message::TaskDiscovered)
    }

    fn fetch_proposals(&self) -> Command<Message> {
        let node = self.node.clone();
        let logger = Arc::clone(&self.logger);

        Command::perform(
            async move {
                if let Some(node) = node {
                    match node.lock().await.list_proposals().await {
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

    /// Starts the GUI including beginning the node run() task, assigning the node the node runner
    /// and agregating node logs.
    fn start_node(&mut self) {
        if let Some(node) = self.node.clone() {
            let (_, rx) = mpsc::channel::<(String, LogLevel)>(100);
            self.toolbar_buttons_state.connect.set_title("Stop Working".to_string());

            tokio::spawn(async move {   
                    let mut cs_node = node.lock().await;
                    let _ = cs_node.initial_setup().await;
                    cs_node.monitor().await;
            });

            self.node_running = true;

            let _ = self.fetch_work_items();

            let _ = Command::perform(
                async move {
                    ReceiverStream::new(rx)
                        .map(|(msg, level)| Message::AddLog(msg, level))
                        .collect::<Vec<_>>()
                        .await
                },
                Message::BatchLog,
            );
        } else {
            let logger = Arc::clone(&self.logger);

            let _ = Command::perform(
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
                |_| Message::Noop,
            );
        }
    }

    fn fetch_work_items(&mut self) -> Command<Message> {
        // let node = self.node.clone();
        // let logger = Arc::clone(&self.logger);

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
                    "status": "GOod"
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
        
        let work_broadcast: WorkBroadcast = serde_json::from_str(work_broadcast_json).unwrap();
        
        let mut itas = Vec::new();
        itas.push(work_broadcast);

        self.validated_task_broadcast = itas;
        Command::none()

        // UNDO

        // Command::perform(
        //     async move {
        //         if let Some(node) = node {
                    
        //             match node.lock().await.list_work_opportunities().await {
        //                 Ok(items) => {
        //                     let logger_guard = logger.lock().await;
        //                     if let Some(logger) = logger_guard.as_ref() {
        //                         logger
        //                             .log(
        //                                 LogLevel::Info,
        //                                 format!("Fetched {} work items", items.len()),
        //                             )
        //                             .await;
        //                     }
        //                     items
        //                 }
        //                 Err(e) => {
        //                     let logger_guard = logger.lock().await;
        //                     if let Some(logger) = logger_guard.as_ref() {
        //                         logger
        //                             .log(
        //                                 LogLevel::Error,
        //                                 format!("Failed to fetch work items: {}", e),
        //                             )
        //                             .await;
        //                     }
        //                     Vec::new()
        //                 }
        //             }
        //         } else {
        //             Vec::new()
        //         }
        //     },
        //     |items| Message::WorkItemsFetched(items),
        // )
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

    // Define settings for the window
    let mut settings = Settings::with_flags(Arc::clone(&logger_arc));
    settings.window.size = (1366, 930); 
    settings.window.resizable = true;
    settings.window.position = Position::Specific(0, 0);

    match GUIState::run(settings) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
