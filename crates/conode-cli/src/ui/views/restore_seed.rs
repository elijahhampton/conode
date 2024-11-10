// src/ui/views/restore_seed.rs
use iced::widget::{text_input, Column, Row, Text};
use iced::widget::{Button, Space};
use iced::{Alignment, Color};
use iced::{Element, Length};

use crate::app::messages::Message;

pub trait RestoreSeedView {
    fn seed_words(&self) -> &Vec<String>;
    fn restore_seed_view(&self) -> Element<Message> {
        let title = Text::new("Restore from Seed Phrase")
            .size(28)
            .style(Color::WHITE);

        let description = Text::new("Enter your 24-word seed phrase in order")
            .size(16)
            .style(Color::from_rgb(0.8, 0.8, 0.8));

        let restore_button = self.create_button(
            "Restore and Start Node",
            Message::RestoreAndStartNode(Vec::new()),
        );

        let mut rows: Vec<Row<Message>> = Vec::new();

        for chunk in (0..24).collect::<Vec<_>>().chunks(3) {
            let row = chunk.iter().fold(Row::new().spacing(10), |row, &i| {
                row.push(
                    text_input(&format!("Word {}", i + 1), &self.seed_words()[i])
                        .on_input(move |value| Message::SeedWordChanged(i, value))
                        .padding(10)
                        .size(14)
                        .width(Length::Fixed(120.0)),
                )
            });
            rows.push(row);
        }

        let content = Column::new()
            .push(title)
            .push(description)
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Column::with_children(
                    rows.into_iter()
                        .map(|row| row.into())
                        .collect::<Vec<Element<_>>>(),
                )
                .spacing(10),
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(restore_button)
            .spacing(10)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;
}
