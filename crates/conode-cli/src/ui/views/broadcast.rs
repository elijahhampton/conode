use iced::widget::{PickList, TextInput};
use iced::{
    alignment::{Horizontal, Vertical},
    theme::{self},
    widget::{Button, Column, Container, Row, Text},
    Alignment, Color, Element, Length,
};

// src/ui/views/broadcast.rs

use crate::app::messages::Message;
use crate::app::state::BroadcastFormState;
use crate::types::enums::ExpiryOption;

pub trait BroadcastView {
    fn broadcast_form(&self) -> &BroadcastFormState;
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;

    fn broadcast_form_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let title = Text::new("Broadcast Work Opportunity")
            .size(28)
            .style(Color::WHITE);

        let expiry_options = PickList::new(
            &[
                ExpiryOption::OneDay,
                ExpiryOption::ThreeDays,
                ExpiryOption::OneWeek,
                ExpiryOption::TwoWeeks,
                ExpiryOption::OneMonth,
            ][..],
            self.broadcast_form().expiry_date,
            Message::ExpiryDateSelected,
        )
        .padding(10)
        .width(Length::Fixed(200.0));

        let reward_input = TextInput::new("Enter reward amount", &self.broadcast_form().reward)
            .padding(10)
            .width(Length::Fixed(200.0))
            .on_input(Message::RewardChanged);

        let requirements_input = TextInput::new(
            "Enter requirements (comma separated)",
            &self.broadcast_form().requirements,
        )
        .padding(10)
        .width(Length::Fixed(400.0))
        .on_input(Message::RequirementsChanged);

        let description_input = Container::new(
            TextInput::new(
                "Enter detailed description",
                &self.broadcast_form().description,
            )
            .padding(10)
            .width(Length::Fixed(400.0))
            .on_input(Message::DescriptionChanged),
        )
        .height(Length::Fixed(150.0));

        let publish_button = Button::new(
            Text::new("Publish to Network")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .width(Length::Fixed(200.0))
        .padding(15)
        .style(theme::Button::Primary)
        .on_press(Message::BroadcastFormSubmit);

        let back_button = self.create_button(
            "Back",
            Message::NavigateTo(crate::types::enums::View::Options),
        );

        let form_content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Expiry Date").size(16))
                    .push(expiry_options)
                    .spacing(5),
            )
            .push(
                Column::new()
                    .push(Text::new("Reward").size(16))
                    .push(reward_input)
                    .spacing(5),
            )
            .push(
                Column::new()
                    .push(Text::new("Requirements").size(16))
                    .push(requirements_input)
                    .spacing(5),
            )
            .push(
                Column::new()
                    .push(Text::new("Description").size(16))
                    .push(description_input)
                    .spacing(5),
            )
            .push(
                Row::new()
                    .push(back_button)
                    .push(publish_button)
                    .spacing(20),
            )
            .spacing(20)
            .padding(20);

        let content = Column::new()
            .push(title)
            .push(form_content)
            .spacing(40)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
