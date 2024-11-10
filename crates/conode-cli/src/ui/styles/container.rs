// src/ui/styles/container.rs
use iced::widget::container;
use iced::{Color, Theme};

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
