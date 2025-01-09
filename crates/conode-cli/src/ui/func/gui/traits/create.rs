use iced::widget::{container, Button, Column, Container, Row, Text};
use iced::Element;
use crate::app::Message;

pub trait CreateComponent {
    /// Creates a task item.
    fn create_task_item<'a>(
        &self,
        title: &'a str,
        payment: &'a str,
        requirements: &'a str,
        time: &'a str,
    ) -> Container<'a, Message>;

    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;

    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;
}