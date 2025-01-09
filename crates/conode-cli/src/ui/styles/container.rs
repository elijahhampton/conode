// src/ui/styles/container.rs
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
pub struct ContentContainerStyle;

impl ContainerStyleSheet for ContentContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.04, 0.05, 0.07))),
            text_color: Some(Color::from_rgb(0.9, 0.9, 0.95)),
            border_radius: 0.0.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OuterContainerStyle;

impl ContainerStyleSheet for OuterContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.02, 0.03, 0.04))),
            text_color: Some(Color::from_rgb(0.9, 0.9, 0.95)),
            ..Default::default()
        }
    }
}