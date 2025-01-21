use iced::{
    theme::{self, Theme},
    widget::{
        button, checkbox, column, container, horizontal_rule, row, scrollable, text, text_input,
        vertical_rule, Button, Column, Container, Row, Space, Text,
    },
    Alignment, Background, Color, Element, Length,
};

use iced::widget::container::StyleSheet as ContainerStyleSheet;

/// Stylesheet for the main view header.
#[derive(Debug, Clone, Copy)]
pub struct HeaderStyle;

impl ContainerStyleSheet for HeaderStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.18))),

            text_color: Some(Color::from_rgb(0.9, 0.9, 0.95)),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..Default::default()
        }
    }
}

/// Stylesheet for the sidebar.
#[derive(Debug, Clone, Copy)]
pub struct SidebarStyle;

impl ContainerStyleSheet for SidebarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.18))),



            border_radius: 2.0.into(),
            text_color: Some(Color::from_rgb(0.9, 0.9, 0.95)),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SectionStyle;

impl ContainerStyleSheet for SectionStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.18))),



            border_radius: 2.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.12, 0.13, 0.15),
            text_color: Some(Color::from_rgb(0.9, 0.9, 0.95)),
            ..Default::default()
        }
    }
}
