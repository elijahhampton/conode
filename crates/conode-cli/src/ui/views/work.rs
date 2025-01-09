// src/ui/views/work.rs

use crate::app::messages::Message;
use crate::ui::styles::button::NegotiationButtonStyle;
use crate::ui::styles::component::WorkItemStyle;
use crate::GUIState;
use conode_types::work::WorkBroadcast;
use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text};
use iced::{alignment, theme, Alignment, Color, Element, Length, Padding, Renderer};
use crate::ui::func::gui::traits::create::CreateComponent;

pub trait WorkView {
    fn work_items(&self) -> &Vec<WorkBroadcast>;
    fn work_opportunities_view(&self) -> Element<Message>
    where
        Self: Sized;
    fn create_work_item_view<'a>(&self, work_broadcast: &'a WorkBroadcast) -> Element<'a, Message>;
}

impl WorkView for GUIState {
    fn work_items(&self) -> &Vec<WorkBroadcast> {
       &self.validated_task_broadcast
    }

    fn create_work_item_view<'a>(&self, work_broadcast: &'a WorkBroadcast) -> Element<'a, Message> {
        let work = &work_broadcast.work;
        let peer_info = &work_broadcast.peer_info;

        let contact_row: Row<'_, Message, Renderer> = Row::new().push(Text::new(work.employer_address.as_deref().unwrap_or("Unknown")).size(14))
        .push(Text::new(peer_info.last_seen.to_string()).size(14));

        let top_row: Row<'_, Message, Renderer> = Row::new().push(contact_row).push(Space::with_width(Length::Fill)).push(Text::new("Expires in 3 days")
        .size(12)
        .style(Color::from_rgb(0.4, 0.6, 1.0)));
    
        Container::new(
            Column::new()
                .push(
                    Row::new().push(top_row)
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(
                            Text::new(&work.details.description)
                                .size(14)
                                .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(
                            Text::new(work.details.requirements.join(", "))
                                .size(14)
                                .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(Text::new("Reward:").size(14).style(Color::WHITE))
                        .push(
                            Text::new(
                                work.details
                                    .reward
                                    .map_or("N/A".to_string(), |r| r.to_string()),
                            )
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10)
                        .push(Space::new(Length::Fixed(0.0.into()), Length::Fixed(5.0.into())))
                )
                .push(Space::new(Length::Fixed(0.0), Length::Fixed(5.0)))
                .push(
                    
                      
                            Button::new(Text::new("Send a proposal").size(16).style(Color::WHITE)).width(Length::FillPortion(1))
                                .style(theme::Button::Custom(Box::new(NegotiationButtonStyle)))
                                .padding([6, 12])
                                .on_press(Message::InitiateNegotiation(work_broadcast.clone())),
                      
                )
                .spacing(5)
                .padding(10),
        )
        .style(theme::Container::Custom(Box::new(WorkItemStyle)))
        .padding(10)
        .width(Length::Fill)
        .into()
    }

    fn work_opportunities_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let instructions_text = Text::new("Your on-chain credentials and wallet address will be used to submit this proposal.")
            .size(14);
        let instructions_container = Container::new(instructions_text)
        .padding(Padding {
            top: 2.0,
            right: 2.0,
            bottom: 2.0,
            left: 2.0,
        })
        .align_y(alignment::Vertical::Center);
        // Work Items Container 
        let work_items_accum = Column::new().spacing(10).padding(10);
    
        // Add actual work items
        let work_items = self.work_items().iter().fold(work_items_accum, |column, item| {
            column.push(self.create_work_item_view(item))
        });
    
        // Scrollable work items
        let scrollable_content = Scrollable::new(work_items)
            .height(Length::Fill)
            .width(Length::Fill);
    
        // Main content
        let content = Column::new()
        .push(Space::new(Length::Fixed(0.0), Length::Fixed(5.0)))  // Add space at top
        .push(instructions_container)
        .push(scrollable_content)
        .spacing(20)
        .align_items(Alignment::Center);
    
        self.create_centered_container(content.into())
    }

}