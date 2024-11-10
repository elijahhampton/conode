// src/ui/views/main.rs
use crate::app::messages::Message;
use iced::widget::{Button, Column, Row, Text};
use iced::{Alignment, Color, Element};

pub trait MainView {
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;

    fn main_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let create_account_button = self.create_button("Create Wallet", Message::CreateAccount);

        let restore_account_button =
            self.create_button("Restore From Seed", Message::RestoreFromSeed);

        let buttons_row = Row::new()
            .push(create_account_button)
            .push(restore_account_button)
            .spacing(20)
            .align_items(Alignment::Center);

        let title_text = Text::new("CoNode").size(35).style(Color::WHITE);

        let subtitle_text = Text::new("A Decentralized and Peer to Peer Network for Work.")
            .size(20)
            .style(Color::WHITE);

        let main_content = Column::new()
            .push(title_text)
            .push(subtitle_text)
            .push(buttons_row)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(main_content.into())
    }
}
