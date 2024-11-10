// src/app/cli.rs

use crate::types::enums::View;
use crate::ui::styles::container::ProposalItemStyle;
use crate::ui::views::active_work::ActiveWorkView;
use crate::ui::views::logs::LogsView;
use crate::ui::views::main::MainView;
use crate::ui::views::options::OptionsView;
use crate::ui::views::proposals::ProposalView;
use crate::ui::views::restore_seed::RestoreSeedView;

use crate::ui::views::work::WorkView;
use conode_types::negotiation::Negotiation;
use conode_types::work::{ActiveWork, JobRole, WorkBroadcast};
use futures::future::BoxFuture;
use iced::widget::{Space, TextInput};
use iced::{
    alignment::{Horizontal, Vertical},
    theme::{self},
    widget::{Button, Column, Container, Row, Scrollable, Text},
    Alignment, Color, Element, Length,
};
use libp2p::PeerId;

use crate::ui::styles::button::{ActionButtonStyle, OutlinedButtonStyle};
use crate::{ui::views::mnemonic::MnemonicView, ConodeCLI};

use super::messages::Message;
use super::state::BroadcastFormState;

impl RestoreSeedView for ConodeCLI {
    fn seed_words(&self) -> &Vec<String> {
        &self.recover_seed_words
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl MnemonicView for ConodeCLI {
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }

    fn mnemonic_words(&self) -> &Vec<String> {
        &self.mnemonic_words
    }
}

use crate::ui::views::broadcast::BroadcastView;

impl BroadcastView for ConodeCLI {
    fn broadcast_form(&self) -> &BroadcastFormState {
        &self.broadcast_form
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl OptionsView for ConodeCLI {
    fn create_option_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::FillPortion(1))
        .height(Length::Fixed(100.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl WorkView for ConodeCLI {
    fn work_items(&self) -> &Vec<WorkBroadcast> {
        &self.work_items
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl MainView for ConodeCLI {
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl LogsView for ConodeCLI {
    fn logger(
        &self,
    ) -> &std::sync::Arc<tokio::sync::Mutex<Option<conode_logging::logger::AsyncLogger>>> {
        &self.logger
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }
}

impl ProposalView for ConodeCLI {
    fn proposals(&self) -> &Vec<Negotiation> {
        &self.proposals
    }

    fn local_peer_id(&self) -> PeerId {
        self.local_peer_id.expect("Local peer ID should be set") // Using the existing field
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }

    fn get_proposal_action(&self, proposal: &Negotiation) -> Option<(String, Message)> {
        let is_employer = self.local_peer_id() == proposal.employer;

        match &proposal.status {
            Some(conode_types::negotiation::ProposalStatus::Proposed) => {
                if is_employer {
                    // Employer can acknowledge the proposal
                    Some((
                        "Acknowledge Proposal".to_string(),
                        Message::AcknowledgeProposal(proposal.clone()),
                    ))
                } else {
                    // Worker just waits
                    None
                }
            }

            Some(conode_types::negotiation::ProposalStatus::Acknowledged) => {
                if is_employer {
                    // Employer can sign the proposal
                    Some((
                        "Sign Proposal".to_string(),
                        Message::SignProposal(proposal.clone()),
                    ))
                } else {
                    None
                }
            }

            Some(conode_types::negotiation::ProposalStatus::EmployerSigned(_)) => {
                if !is_employer {
                    // Worker can countersign
                    Some((
                        "Sign & Confirm".to_string(),
                        Message::SignAndConfirmProposal(proposal.clone()),
                    ))
                } else {
                    None
                }
            }

            Some(conode_types::negotiation::ProposalStatus::FullySigned { .. }) => {
                if is_employer {
                    // Employer can create the work on chain
                    Some((
                        "Create Work".to_string(),
                        Message::CreateWorkOnChain(proposal.clone()),
                    ))
                } else {
                    None
                }
            }

            Some(conode_types::negotiation::ProposalStatus::Error(_error)) => {
                // Show error status but no action
                None
            }

            None => None,
        }
    }

    fn create_proposal_item_view<'a>(&self, proposal: &'a Negotiation) -> Element<'a, Message> {
        let is_employer = self.local_peer_id() == proposal.employer;
        let role_text = if is_employer { "Employer" } else { "Worker" };

        let status_color = match &proposal.status {
            Some(conode_types::negotiation::ProposalStatus::Proposed) => {
                Color::from_rgb(1.0, 0.8, 0.0)
            }
            Some(conode_types::negotiation::ProposalStatus::Acknowledged) => {
                Color::from_rgb(0.0, 0.8, 1.0)
            }
            Some(conode_types::negotiation::ProposalStatus::EmployerSigned(_)) => {
                Color::from_rgb(0.5, 0.8, 0.5)
            }
            Some(conode_types::negotiation::ProposalStatus::FullySigned { .. }) => {
                Color::from_rgb(0.0, 1.0, 0.0)
            }
            Some(conode_types::negotiation::ProposalStatus::Error(_)) => {
                Color::from_rgb(1.0, 0.0, 0.0)
            }
            None => Color::from_rgb(0.5, 0.5, 0.5),
        };

        let mut content = Column::new()
            .push(
                Row::new()
                    .push(Text::new("Proposal ID:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&proposal.id)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Job ID:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&proposal.job_id)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Role:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(role_text)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Status:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(std::format!("{:?}", proposal.status))
                            .size(14)
                            .style(status_color),
                    )
                    .spacing(10),
            );

        if let Some(payout) = proposal.proposed_payout {
            content = content.push(
                Row::new()
                    .push(Text::new("Proposed Payout:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(payout.to_string())
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );
        }

        // Add action button if available
        if let Some((button_text, message)) = self.get_proposal_action(proposal) {
            content = content.push(
                Row::new()
                    .push(iced::widget::Space::with_width(Length::Fill))
                    .push(
                        Button::new(Text::new(button_text).size(12).style(Color::WHITE))
                            .style(theme::Button::Custom(Box::new(
                                crate::ui::styles::button::ActionButtonStyle,
                            )))
                            .padding([6, 12])
                            .on_press(message),
                    ),
            );
        }

        Container::new(content)
            .style(theme::Container::Custom(Box::new(
                crate::ui::styles::container::ProposalItemStyle,
            )))
            .padding(10)
            .width(Length::Fill)
            .into()
    }

    fn proposals_view(&self) -> Element<Message> {
        let mut proposals_list = Column::new().spacing(10).padding(10);

        // Add proposal items
        proposals_list = self
            .proposals()
            .iter()
            .fold(proposals_list, |column, proposal| {
                column.push(self.create_proposal_item_view(proposal))
            });

        let scrollable_content = Scrollable::new(proposals_list)
            .height(Length::Fill)
            .width(Length::Fill);

        // Use OptionsView's create_option_button explicitly
        let back_button = <Self as OptionsView>::create_option_button(
            self,
            "Back",
            Message::NavigateTo(crate::types::enums::View::Options),
        );

        let content = Column::new()
            .push(Text::new("Active Proposals").size(24).style(Color::WHITE))
            .push(scrollable_content)
            .push(back_button)
            .spacing(20)
            .align_items(Alignment::Center);

        // Use ProposalView's create_centered_container explicitly
        <Self as ProposalView>::create_centered_container(self, content.into())
    }
}

impl ActiveWorkView for ConodeCLI {
    fn active_works(&self) -> &Vec<ActiveWork> {
        &self.active_works
    }

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(content)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(600.0))
            .center_x()
            .center_y()
            .into()
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message> {
        Button::new(
            Text::new(label)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Custom(Box::new(OutlinedButtonStyle)))
        .on_press(message)
    }

    fn get_solution(
        &self,
        work_id: String,
    ) -> BoxFuture<'_, Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>> {
        Box::pin(async move {
            if let Some(node) = &self.node {
                node.solution(work_id).await
            } else {
                Ok(None)
            }
        })
    }

    fn get_work_action(
        &self,
        work: ActiveWork,
    ) -> BoxFuture<'_, Result<Option<(String, Message)>, Box<dyn std::error::Error + Send + Sync>>>
    {
        Box::pin(async move {
            match work.role {
                JobRole::Worker => {
                    let solution = self.get_solution(work.work.id.clone()).await?;
                    Ok(if solution.is_none() {
                        Some(("Submit Solution".to_string(), Message::SubmitSolution(work)))
                    } else {
                        None
                    })
                }
                JobRole::Employer => {
                    let solution = self.get_solution(work.work.id.clone()).await?;
                    Ok(if solution.is_some() {
                        Some(("Review Solution".to_string(), Message::ReviewSolution(work)))
                    } else {
                        None
                    })
                }
            }
        })
    }

    fn create_active_work_item_view<'a>(
        &'a self, // Add lifetime annotation here
        active_work: &'a ActiveWork,
    ) -> Element<'a, Message> {
        let role_text = match active_work.role {
            JobRole::Employer => "Employer",
            JobRole::Worker => "Worker",
        };

        let mut content = Column::new()
            .push(
                Row::new()
                    .push(Text::new("Work ID:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&active_work.work.id)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Role:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(role_text)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Reward:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(active_work.work.reward.to_string())
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );

        // Add solution input and submit button for worker

        if self
            .active_work_solutions_submitted
            .get(&active_work.work.id.clone())
            .is_some_and(|val| val.is_some())
            && matches!(active_work.role, JobRole::Employer)
        {
            let solution = self
                .active_work_solutions_submitted
                .get(&active_work.work.id.clone())
                .unwrap()
                .as_ref()
                .unwrap();
            let row = Row::new()
                .push(Text::new("Solution:  "))
                .push(Text::new(solution.to_string()));
            content = content.push(row).push(
                Button::new(Text::new("Accept Solution").horizontal_alignment(Horizontal::Center))
                    .on_press(Message::SolutionTestedInternally(
                        active_work.work.id.clone(),
                        *solution,
                    )),
            );
        }

        if matches!(active_work.role, JobRole::Worker) {
            content = content.push(Space::with_height(Length::Fixed(10.0))).push(
                Column::new()
                    .push(
                        TextInput::new(
                            "Enter solution...",
                            &self
                                .active_work_solutions
                                .get(&active_work.work.id)
                                .cloned()
                                .unwrap_or_default(),
                        )
                        .padding(10)
                        .size(14)
                        .on_input(move |s| {
                            Message::SolutionChanged(active_work.work.id.clone(), s)
                        }),
                    )
                    .push(Space::with_height(Length::Fixed(10.0)))
                    .push(
                        Button::new(
                            Text::new("Submit Solution").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Fill)
                        .padding(10)
                        .style(theme::Button::Custom(Box::new(ActionButtonStyle)))
                        .on_press(Message::SubmitSolution(active_work.clone())),
                    ),
            );
        }

        Container::new(content)
            .style(theme::Container::Custom(Box::new(ProposalItemStyle)))
            .padding(10)
            .width(Length::Fill)
            .into()
    }

    fn active_work_view(&self) -> Element<Message> {
        let mut active_works_list = Column::new().spacing(10).padding(10);

        active_works_list = self
            .active_works()
            .iter()
            .fold(active_works_list, |column, work| {
                column.push(self.create_active_work_item_view(work))
            });

        let scrollable_content = Scrollable::new(active_works_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let back_button =
            ActiveWorkView::create_button(self, "Back", Message::NavigateTo(View::Options));

        let content = Column::new()
            .push(Text::new("Active Work").size(24).style(Color::WHITE))
            .push(scrollable_content)
            .push(back_button)
            .spacing(20)
            .align_items(Alignment::Center);

        ActiveWorkView::create_centered_container(self, content.into())
    }
}
