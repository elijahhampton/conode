// src/ui/styles/button.rs
use iced::widget::button::StyleSheet as ButtonStyleSheet;
use iced::{
    theme::{self, Theme},
    widget::button,
    Color,
};
use iced::{Background, Vector};

/// Stylesheet for main toolbar buttons.
#[derive(Debug, Clone, Copy)]
pub struct ModernButtonStyle;

impl ButtonStyleSheet for ModernButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
            border_color: Color::from_rgb(0.25, 0.25, 0.25),
            border_radius: 2.0.into(), // Smaller radius
            border_width: 1.0,
            text_color: Color::from_rgb(0.9, 0.9, 0.9),
            shadow_offset: iced::Vector::new(0.0, 0.0),

            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.22, 0.22, 0.22))),
            border_color: Color::from_rgb(0.3, 0.3, 0.3),
            ..self.active(_style)
        }
    }
}

pub struct OutlinedButtonStyle;

impl button::StyleSheet for OutlinedButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Color::TRANSPARENT.into()),
            border_radius: 4.0.into(),
            border_width: 1.0,
            border_color: Color::WHITE,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
            text_color: Color::WHITE,
            ..active
        }
    }
}

pub struct PrimaryButtonStyle;

impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Color::from_rgb(0.2, 0.5, 1.0).into()),
            border_radius: 4.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Color::from_rgb(0.3, 0.6, 1.0).into()),
            ..active
        }
    }
}
pub struct NegotiationButtonStyle;

impl button::StyleSheet for NegotiationButtonStyle {
    type Style = theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.17, 0.2))), // Dark grayish blue
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.2, 0.22, 0.25), // Slightly lighter border
            text_color: Color::WHITE,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.18, 0.2, 0.23))), // Slightly lighter when hovered
            border_color: Color::from_rgb(0.23, 0.25, 0.28), // Slightly lighter border on hover
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.13, 0.15, 0.18))), // Slightly darker when pressed
            border_color: Color::from_rgb(0.18, 0.2, 0.23), // Darker border when pressed
            ..active
        }
    }
}

pub struct ActionButtonStyle;

impl button::StyleSheet for ActionButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Color::from_rgb(0.2, 0.5, 0.8).into()),
            border_radius: 4.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.3, 0.6, 0.9),
            text_color: Color::WHITE,
            shadow_offset: Vector::new(0.0, 0.0),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Color::from_rgb(0.25, 0.55, 0.85).into()),
            ..active
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Color::from_rgb(0.15, 0.45, 0.75).into()),
            shadow_offset: Vector::new(0.0, 0.0),
            ..active
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        button::Appearance {
            background: Some(Color::from_rgb(0.1, 0.1, 0.1).into()),
            text_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..active
        }
    }
}
