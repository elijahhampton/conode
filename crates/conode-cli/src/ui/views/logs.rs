// src/ui/views/logs.rs

use crate::app::messages::Message;
use crate::types::enums::View;
use crate::ui::func::gui::traits::create::CreateComponent;
use crate::GUIState;
use chrono::{Local, Utc};
use conode_logging::logger::{LogEntry, LogLevel};
use futures::executor;
use iced::widget::{Button, Column, Scrollable, Text};
use iced::{Alignment, Color, Element, Length};
use tokio::sync::Mutex;

pub trait LogsView {
    fn logger(&self) -> &std::sync::Arc<Mutex<Option<conode_logging::logger::AsyncLogger>>>;
    fn logs_view(&self) -> Element<Message>
    where
        Self: Sized;
}

impl LogsView for GUIState {
    fn logger(
        &self,
    ) -> &std::sync::Arc<tokio::sync::Mutex<Option<conode_logging::logger::AsyncLogger>>> {
        &self.logger
    }

    fn logs_view(&self) -> Element<Message>
    where
        Self: Sized,
    {
        let title = Text::new("Logs").size(28).style(Color::WHITE);

        let logs = executor::block_on(async {
            if let Some(logger) = self.logger().lock().await.as_ref() {
                logger.get_logs().await
            } else {
                vec![LogEntry {
                    timestamp: Utc::now(),
                    level: LogLevel::Info,
                    message: "Logger not initialized".to_string(),
                }]
            }
        });

        let logs_list = logs
            .into_iter()
            .fold(Column::new().spacing(10), |column, log| {
                column.push(
                    Text::new(format!(
                        "[{}] {:?}: {}",
                        log.timestamp
                            .with_timezone(&Local)
                            .format("%Y-%m-%d %H:%M:%S"),
                        log.level,
                        log.message
                    ))
                    .size(14)
                    .style(match log.level {
                        LogLevel::Error => Color::from_rgb(1.0, 0.4, 0.4),
                        LogLevel::Warning => Color::from_rgb(1.0, 0.8, 0.2),
                        _ => Color::WHITE,
                    }),
                )
            });

        let scrollable_logs = Scrollable::new(logs_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let content = Column::new()
            .push(title)
            .push(scrollable_logs)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
