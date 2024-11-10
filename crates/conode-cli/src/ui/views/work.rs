// src/ui/views/work.rs

use crate::app::messages::Message;
use crate::types::enums::View;
use crate::ui::styles::button::NegotiationButtonStyle;
use crate::ui::styles::container::WorkItemStyle;
use conode_types::work::WorkBroadcast;
use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text};
use iced::{theme, Alignment, Color, Element, Length};

pub trait WorkView {
    fn work_items(&self) -> &Vec<WorkBroadcast>;
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;

    fn create_work_item_view<'a>(&self, work_broadcast: &'a WorkBroadcast) -> Element<'a, Message> {
        let work = &work_broadcast.work;
        let peer_info = &work_broadcast.peer_info;

        Container::new(
            Column::new()
                .push(
                    Row::new()
                        .push(Text::new("Description:").size(16).style(Color::WHITE))
                        .push(
                            Text::new(&work.details.description)
                                .size(14)
                                .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(Text::new("Requirements:").size(14).style(Color::WHITE))
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
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(Text::new("Employer:").size(14).style(Color::WHITE))
                        .push(
                            Text::new(work.employer_address.as_deref().unwrap_or("Unknown"))
                                .size(14)
                                .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(Text::new("Last Seen:").size(14).style(Color::WHITE))
                        .push(
                            Text::new(peer_info.last_seen.to_string())
                                .size(14)
                                .style(Color::from_rgb(0.8, 0.8, 0.8)),
                        )
                        .spacing(10),
                )
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Fill)) // Push button to the right
                        .push(
                            Button::new(Text::new("ation").size(12).style(Color::WHITE))
                                .style(theme::Button::Custom(Box::new(NegotiationButtonStyle)))
                                .padding([6, 12])
                                .on_press(Message::InitiateNegotiation(work_broadcast.clone())),
                        ),
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
        // Work Items Container
        let work_items = Column::new().spacing(10).padding(10);

        // Add actual work items
        let work_items = self.work_items().iter().fold(work_items, |column, item| {
            column.push(self.create_work_item_view(item))
        });

        // Scrollable work items
        let scrollable_content = Scrollable::new(work_items)
            .height(Length::Fill)
            .width(Length::Fill);

        // Back button
        let back_button = self.create_button("Back", Message::NavigateTo(View::Options));

        // Main content
        let content = Column::new()
            .push(scrollable_content)
            .push(back_button)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
