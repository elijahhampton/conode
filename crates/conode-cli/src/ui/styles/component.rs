
use iced::widget::container::StyleSheet as ContainerStyleSheet;
use iced::{
    theme::{self, Theme},
    widget::{
        button, checkbox, column, container, horizontal_rule, row, scrollable, text, text_input,
        vertical_rule, Button, Column, Container, Row, Space, Text,
    },
    Alignment, Background, Color, Element, Length,
};

#[derive(Debug, Clone, Copy)]
pub struct TaskItemStyle;

impl container::StyleSheet for TaskItemStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: None,
            background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.15))),
            border_radius: 6.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.2, 0.2, 0.25),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PaymentBadgeStyle;

impl container::StyleSheet for PaymentBadgeStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.2, 0.8, 0.4)),
            background: Some(Background::Color(Color::from_rgba(0.2, 0.8, 0.4, 0.1))),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::from_rgba(0.2, 0.8, 0.4, 0.2),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StatusBadgeStyle;

impl ContainerStyleSheet for StatusBadgeStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.2, 0.9, 0.4)),
            background: Some(Background::Color(Color::from_rgb(0.05, 0.05, 0.07))),
            border_radius: 2.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.2, 0.9, 0.4),
            ..Default::default()
        }
    }
}

pub struct WorkItemStyle;

impl container::StyleSheet for WorkItemStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Color::from_rgb(0.2, 0.2, 0.2).into()),
            border_radius: 4.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.3, 0.3, 0.3),
        }
    }
}

pub struct ProposalItemStyle;

impl container::StyleSheet for ProposalItemStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Color::from_rgb(0.15, 0.15, 0.15).into()),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.3, 0.3, 0.3),
        }
    }
}

pub fn sync_status_container(theme: &Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
        border_radius: 0.0,
        border_width: 1.0,
        border_color: iced::Color::from_rgb(0.2, 0.2, 0.2),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyncStatusStyle;

impl container::StyleSheet for SyncStatusStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.8, 0.8, 0.8)),  // Light grey text
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))), // Dark grey background
            border_radius: 0.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.2, 0.2, 0.2),
            ..Default::default()
        }
    }
}