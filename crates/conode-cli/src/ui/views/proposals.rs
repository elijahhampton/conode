use crate::app::messages::Message;
use crate::types::enums::View;
use crate::ui::styles::button::{ActionButtonStyle, OutlinedButtonStyle};
use crate::ui::styles::container::ProposalItemStyle;
use conode_types::negotiation::{Negotiation, ProposalStatus};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text};
use iced::{theme, Alignment, Color, Element, Length};
use libp2p::PeerId;

pub trait ProposalView {
    fn proposals(&self) -> &Vec<Negotiation>;
    fn local_peer_id(&self) -> PeerId;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;

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

    fn get_proposal_action(&self, proposal: &Negotiation) -> Option<(String, Message)> {
        let is_employer = self.local_peer_id() == proposal.employer;

        match &proposal.status {
            Some(ProposalStatus::Proposed) => {
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

            Some(ProposalStatus::Acknowledged) => {
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

            Some(ProposalStatus::EmployerSigned(_)) => {
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

            Some(ProposalStatus::FullySigned { .. }) => {
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

            Some(ProposalStatus::Error(_error)) => {
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
            Some(ProposalStatus::Proposed) => Color::from_rgb(1.0, 0.8, 0.0),
            Some(ProposalStatus::Acknowledged) => Color::from_rgb(0.0, 0.8, 1.0),
            Some(ProposalStatus::EmployerSigned(_)) => Color::from_rgb(0.5, 0.8, 0.5),
            Some(ProposalStatus::FullySigned { .. }) => Color::from_rgb(0.0, 1.0, 0.0),
            Some(ProposalStatus::Error(_)) => Color::from_rgb(1.0, 0.0, 0.0),
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
                        Text::new(format!("{:?}", proposal.status))
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
                Row::new().push(Space::with_width(Length::Fill)).push(
                    Button::new(Text::new(button_text).size(12).style(Color::WHITE))
                        .style(theme::Button::Custom(Box::new(ActionButtonStyle)))
                        .padding([6, 12])
                        .on_press(message),
                ),
            );
        }

        Container::new(content)
            .style(theme::Container::Custom(Box::new(ProposalItemStyle)))
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

        let back_button = self.create_button("Back", Message::NavigateTo(View::Options));

        let content = Column::new()
            .push(Text::new("Active Proposals").size(24).style(Color::WHITE))
            .push(scrollable_content)
            .push(back_button)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
