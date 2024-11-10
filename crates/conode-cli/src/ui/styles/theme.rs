// src/ui/styles/theme.rs

use iced::Color;
pub struct _ThemeColors;

impl _ThemeColors {
    // Primary colors
    pub const _PRIMARY: Color = Color::from_rgb(0.2, 0.5, 1.0);
    pub const _PRIMARY_DARK: Color = Color::from_rgb(0.1, 0.4, 0.9);
    pub const _PRIMARY_LIGHT: Color = Color::from_rgb(0.3, 0.6, 1.0);

    // Background colors
    pub const _BACKGROUND: Color = Color::from_rgb(0.1, 0.1, 0.1);
    pub const _SURFACE: Color = Color::from_rgb(0.2, 0.2, 0.2);

    // Text colors
    pub const _TEXT_PRIMARY: Color = Color::WHITE;
    pub const _TEXT_SECONDARY: Color = Color::from_rgb(0.8, 0.8, 0.8);

    // State colors
    pub const _ERROR: Color = Color::from_rgb(1.0, 0.4, 0.4);
    pub const _WARNING: Color = Color::from_rgb(1.0, 0.8, 0.2);
    pub const _SUCCESS: Color = Color::from_rgb(0.2, 0.8, 0.2);
}

// Spacing constants
pub struct _Spacing;

impl _Spacing {
    pub const _NONE: f32 = 0.0;
    pub const _TINY: f32 = 4.0;
    pub const _SMALL: f32 = 8.0;
    pub const _MEDIUM: f32 = 16.0;
    pub const _LARGE: f32 = 24.0;
    pub const _XLARGE: f32 = 32.0;
}

// Size constants
pub struct _Sizes;

impl _Sizes {
    pub const _BUTTON_WIDTH: f32 = 200.0;
    pub const _BUTTON_HEIGHT: f32 = 40.0;
    pub const _BUTTON_PADDING: f32 = 15.0;

    pub const _CONTAINER_WIDTH: f32 = 600.0;
    pub const _CONTAINER_HEIGHT: f32 = 600.0;

    pub const _TEXT_SMALL: u16 = 14;
    pub const _TEXT_MEDIUM: u16 = 16;
    pub const _TEXT_LARGE: u16 = 20;
    pub const _TEXT_XLARGE: u16 = 28;
    pub const _TEXT_TITLE: u16 = 35;
}

// Border radius constants
pub struct _BorderRadius;

impl _BorderRadius {
    pub const _SMALL: f32 = 4.0;
    pub const _MEDIUM: f32 = 8.0;
    pub const _LARGE: f32 = 12.0;
    pub const _ROUND: f32 = 9999.0;
}
