// src/ui/views/mnemonic.rs

use crate::app::messages::Message;
use crate::ui::func::gui::traits::create::CreateComponent;
use crate::GUIState;
use iced::alignment::Alignment;
use iced::theme;
use iced::widget::{Button, Column, Container, Row, Text};
use iced::{Color, Element, Length};

pub trait MnemonicView {
    fn mnemonic_words(&self) -> &Vec<String>;

    fn mnemonic_view(&self) -> Element<Message>
    where
        Self: Sized;
}

impl MnemonicView for GUIState {
    fn mnemonic_words(&self) -> &Vec<String> {
        &self.mnemonic_words
    }

    fn mnemonic_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let title = Text::new("Your Private Recovery Phrase")
            .size(28)
            .style(Color::WHITE);

        let description = Text::new("Below are the 24 words that protect your assets. Write them down exactly as shown and store them somewhere safe. Remember: anyone with these words can access your wallet.")
        .size(20)
        .style(Color::WHITE)
        .horizontal_alignment(iced::alignment::Horizontal::Center) // Centers text horizontally
        .width(Length::Fill);

        let title_column = Column::new()
            .push(title)
            .push(description)
            .align_items(Alignment::Center)
            .padding(40)
            .spacing(15);

        let mnemonic_grid: Element<Message> = Column::with_children(
            self.mnemonic_words()
                .chunks(4)
                .enumerate()
                .map(|(row_index, chunk)| {
                    Row::with_children(
                        chunk
                            .iter()
                            .enumerate()
                            .map(|(col_index, word)| {
                                let index = row_index * 4 + col_index + 1;
                                Container::new(
                                    Text::new(format!("{}. {}", index, word))
                                        .size(16)
                                        .style(Color::WHITE),
                                )
                                .width(Length::Fill)
                                .padding(10)
                                .style(theme::Container::Box)
                                .into()
                            })
                            .collect(),
                    )
                    .spacing(10)
                    .into()
                })
                .collect(),
        )
        .spacing(10)
        .padding(20)
        .into();

        let start_node_button = self.create_button(
            "I made a copy of the 24 word mnemonic.",
            Message::OptionsView,
        );

        let content = Column::new()
            .push(title_column)
            .push(mnemonic_grid)
            .push(start_node_button)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
