// src/ui/views/options.rs

use crate::app::messages::Message;
use crate::ui::func::gui::traits::create::CreateComponent;
use crate::ui::styles::button::OutlinedButtonStyle;
use crate::GUIState;
use iced::theme;
use iced::widget::{Button, Column, Row, Text};
use iced::{
    alignment::{Horizontal, Vertical},
    Alignment, Element, Length,
};

pub trait OptionsView {
    fn create_option_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
    fn options_view(&self) -> Element<Message>
    where
        Self: Sized;
}

impl OptionsView for GUIState {
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

    fn options_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let logs_button = self.create_option_button("Activity Feed", Message::ViewLogs);

        let opportunities_button =
            self.create_option_button("View Opportunities", Message::ViewOpportunities);

        let broadcast_button = self.create_option_button("Publish Work", Message::BroadcastWork);

        let proposals_button = self.create_option_button("Proposals", Message::ViewProposals);

        let active_work_button = self.create_option_button("Active Work", Message::ViewActiveWork);

        let completed_work_button =
            self.create_option_button("Completed Work", Message::ViewCompletedWork);

        let administation_button_row = Row::new()
            .push(logs_button)
            .push(opportunities_button)
            .push(broadcast_button)
            .spacing(10);

        let proposal_button_row = Row::new().push(proposals_button).spacing(10);

        let work_button_row = Row::new()
            .push(active_work_button)
            .push(completed_work_button)
            .spacing(10);

        let options_grid = Column::new()
            .push(administation_button_row)
            .push(proposal_button_row)
            .push(work_button_row)
            .spacing(10)
            .align_items(Alignment::Center);

        let content = Column::new()
            .push(options_grid)
            .spacing(40)
            .padding(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
