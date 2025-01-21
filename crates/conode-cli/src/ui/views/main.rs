use crate::GUIState;
use crate::{
    app::messages::Message,
    ui::{
        constant::{application_title_text, APPLICATION_VERSION},
        func::gui::traits::create::CreateComponent,
        styles::{
            common::{HeaderStyle, SectionStyle, SidebarStyle},
            component::StatusBadgeStyle,
            container::ContentContainerStyle,
        },
    },
};
use chrono::{Local, Utc};
use conode_logging::logger::{LogEntry, LogLevel};
use futures::executor;
use iced::{
    theme::{self, Theme},
    widget::{checkbox, container, scrollable, Column, Container, Row, Space, Text},
    Background, Color, Element, Length,
};

use iced::widget::container::StyleSheet as ContainerStyleSheet;
use iced_native::Pixels;

use super::{broadcast::BroadcastView, proposals::ProposalView};

#[derive(Debug, Clone, PartialEq)]
pub enum MainContentView {
    ViewTasks,
    PublishTask,
    ViewDetails,
    DiscoverTask,
    ViewProposals,
}

pub trait MainView {
    fn main_view(&self) -> Element<Message>
    where
        Self: Sized;
}

impl MainView for GUIState {
    fn main_view(&self) -> Element<Message> {
        let header = Container::new(
            Row::new()
                .push(
                    Column::new()
                        .push(Text::new(application_title_text()).size(18))
                        .push(Text::new(APPLICATION_VERSION).size(12)),
                )
                .push(Space::with_width(Length::Fill))
                .push(
                    Container::new(Text::new("●  Connected").size(14))
                        .style(theme::Container::Custom(Box::new(StatusBadgeStyle)))
                        .padding(10),
                ),
        )
        .padding(15)
        .style(theme::Container::Custom(Box::new(HeaderStyle)));

        let sidebar = Container::new(
            Column::new()
                .push(
                    Container::new(
                        Column::new()
                            .push(Text::new("Network Identity").size(16))
                            .push(Text::new("PeerID: 0x7f..3d").size(14)),
                    )
                    .width(Length::FillPortion(1))
                    .style(theme::Container::Custom(Box::new(SectionStyle)))
                    .padding(15),
                )
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(
                    Container::new(
                        Column::new()
                            .push(Text::new("Markets").size(16))
                            .push(
                                checkbox("Development", true, |b| {
                                    Message::FilterMarket("development".into(), b)
                                })
                                .size(14)
                                .text_size(14),
                            )
                            .push(
                                checkbox("Design", true, |b| {
                                    Message::FilterMarket("design".into(), b)
                                })
                                .size(14)
                                .text_size(14),
                            )
                            .push(
                                checkbox("Writing", true, |b| {
                                    Message::FilterMarket("writing".into(), b)
                                })
                                .size(14)
                                .text_size(14),
                            )
                            .push(
                                checkbox("Marketing", true, |b| {
                                    Message::FilterMarket("marketing".into(), b)
                                })
                                .size(14)
                                .text_size(14),
                            )
                            .spacing(8),
                    )
                    .width(Length::FillPortion(1))
                    .style(theme::Container::Custom(Box::new(SectionStyle)))
                    .padding(15),
                )
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(
                    Container::new(
                        Column::new()
                            .push(Text::new("Network Stats").size(16))
                            .push(Text::new("Peers: 45").size(14))
                            .push(Text::new("Tasks: 128").size(14))
                            .push(Text::new("Markets: 4").size(14))
                            .spacing(8),
                    )
                    .width(Length::FillPortion(1))
                    .style(theme::Container::Custom(Box::new(SectionStyle)))
                    .padding(15),
                ),
        )
        .width(Length::Fixed(250.0))
        .style(theme::Container::Custom(Box::new(SidebarStyle)))
        .padding(15);

        let search_bar = Container::new(
            Row::new()
                .push(self.create_button(
                    &self.toolbar_buttons_state.connect.title,
                    self.toolbar_buttons_state.connect.action.clone(),
                ))
                .push(self.create_button(
                    &self.toolbar_buttons_state.discover.title,
                    self.toolbar_buttons_state.discover.action.clone(),
                ))
                .push(self.create_button(
                    &self.toolbar_buttons_state.publish.title,
                    self.toolbar_buttons_state.publish.action.clone(),
                ))
                .push(self.create_button(
                    &self.toolbar_buttons_state.view_proposals.title,
                    self.toolbar_buttons_state.view_proposals.action.clone(),
                ))
                .push(self.create_button(
                    &self.toolbar_buttons_state.view_task.title,
                    self.toolbar_buttons_state.view_task.action.clone(),
                ))
                .spacing(15),
        )
        .style(theme::Container::Custom(Box::new(SectionStyle)))
        .width(Length::FillPortion(1))
        .padding(15);

    let row_one = Row::new().spacing(Pixels::from(20)).push(self.create_task_item(
        "Debug and patch MEV attack vector in lending protocol. High priority. Need to implement proper slippage checks and oracle validation sequence.",
        "24.5 ETH",
        "Solidity, MEV",
        "15m ago"
    ))
    .push(self.create_task_item(
        "Port existing Vyper DEX contracts to Cairo for L2 deployment. Must maintain exact decimal precision and handle all edge cases.",
        "18.2 ETH",
        "Cairo, Vyper",
        "1h ago"
    ))
    .push(self.create_task_item(
        "Implement account abstraction wallet with fallback handlers. Need social recovery and daily limits.",
        "7.8 ETH",
        "ERC-4337, Solidity", 
        "3h ago"
    ));

        let task_list = Container::new(
            scrollable(
                Column::new()
                .push(row_one)
                    .spacing(10)
            )
        )
        .height(Length::Fill)
        .style(theme::Container::Custom(Box::new(SectionStyle)))
        .padding(15);

        let main_content = Column::new()
            .push(search_bar)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(match self.current_main_view {
                MainContentView::ViewTasks => task_list.into(),
                MainContentView::PublishTask => self.broadcast_form_view(),
                MainContentView::ViewDetails => Container::new(Text::new("Details View")).into(),
                MainContentView::DiscoverTask => self.network_task_view(),
                MainContentView::ViewProposals => self.proposals_view(),
            })
            .width(Length::Fill)
            .spacing(0);

        let logs = executor::block_on(async {
            if let Some(logger) = self.logger.lock().await.as_ref() {
                logger.get_logs().await
            } else {
                vec![LogEntry {
                    timestamp: Utc::now(),
                    level: LogLevel::Info,
                    message: "CoNode interface started.".to_string(),
                }]
            }
        });

        let logs_panel = Container::new(scrollable(logs.into_iter().fold(
            Column::new().spacing(8),
            |column, log| {
                column.push(
                    Row::new()
                        .push(
                            Text::new(format!(
                                "[{}]",
                                log.timestamp.with_timezone(&Local).format("%H:%M:%S")
                            ))
                            .size(12)
                            .style(Color::from_rgb(0.5, 0.5, 0.5)),
                        )
                        .push(Space::with_width(Length::Fixed(8.0)))
                        .push(Text::new(format!("{}", log.message)).size(14).style(
                            match log.level {
                                LogLevel::Error => Color::from_rgb(1.0, 0.4, 0.4),
                                LogLevel::Warning => Color::from_rgb(1.0, 0.8, 0.2),
                                _ => Color::from_rgb(0.7, 0.7, 0.7),
                            },
                        )),
                )
            },
        )))
        .width(Length::Fixed(300.0))
        .height(Length::Fill)
        .style(theme::Container::Custom(Box::new(LogsStyle)))
        .padding(15);

        let body = Container::new(
            Row::new()
                .push(sidebar)
                .push(Space::with_width(Length::Fixed(15.0)))
                .push(main_content)
                .push(Space::with_width(Length::Fixed(15.0)))
                .push(logs_panel),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(15)
        .style(theme::Container::Custom(Box::new(ContentContainerStyle)));

        let content = Column::new()
            .push(header)
            .push(body)
            .width(Length::Fill)
            .height(Length::Fill);

        self.create_centered_container(content.into())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogsStyle;

impl ContainerStyleSheet for LogsStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.18))),

            border_radius: 2.0.into(),
            text_color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
            ..Default::default()
        }
    }
}
