//! Visual theme matching the lamco brand palette.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme};

/// Colors follow a neutral base with blue accent, matching admin console branding.
pub mod colors {
    use iced::Color;

    pub const PRIMARY: Color = Color::from_rgb(0.2, 0.4, 0.8);
    pub const PRIMARY_LIGHT: Color = Color::from_rgb(0.4, 0.6, 0.9);
    pub const PRIMARY_DARK: Color = Color::from_rgb(0.1, 0.3, 0.6);

    pub const SURFACE: Color = Color::from_rgb(1.0, 1.0, 1.0);
    pub const SURFACE_DARK: Color = Color::from_rgb(0.96, 0.96, 0.98);
    pub const BACKGROUND: Color = Color::from_rgb(0.94, 0.94, 0.96);

    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.1, 0.1, 0.15);
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.4, 0.4, 0.5);
    pub const TEXT_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.65);

    pub const SUCCESS: Color = Color::from_rgb(0.2, 0.7, 0.3);
    pub const WARNING: Color = Color::from_rgb(0.9, 0.6, 0.0);
    pub const ERROR: Color = Color::from_rgb(0.9, 0.2, 0.2);
    pub const INFO: Color = Color::from_rgb(0.2, 0.5, 0.9);

    /// Service registry levels - green/amber/yellow/gray for at-a-glance status
    pub const GUARANTEED: Color = Color::from_rgb(0.0, 0.7, 0.3);
    pub const BEST_EFFORT: Color = Color::from_rgb(1.0, 0.6, 0.0);
    pub const DEGRADED: Color = Color::from_rgb(0.9, 0.7, 0.0);
    pub const UNAVAILABLE: Color = Color::from_rgb(0.5, 0.5, 0.5);

    pub const LOG_ERROR: Color = Color::from_rgb(0.9, 0.0, 0.0);
    pub const LOG_WARN: Color = Color::from_rgb(0.9, 0.7, 0.0);
    pub const LOG_INFO: Color = Color::from_rgb(0.0, 0.0, 0.0);
    pub const LOG_DEBUG: Color = Color::from_rgb(0.3, 0.3, 0.8);
    pub const LOG_TRACE: Color = Color::from_rgb(0.5, 0.5, 0.5);

    pub const TAB_ACTIVE: Color = PRIMARY;
    pub const TAB_INACTIVE: Color = Color::from_rgb(0.7, 0.7, 0.75);
    pub const TAB_HOVER: Color = PRIMARY_LIGHT;
}

pub fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(colors::PRIMARY)),
        text_color: Color::WHITE,
        border: Border {
            color: colors::PRIMARY_DARK,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(colors::PRIMARY_LIGHT)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(colors::PRIMARY_DARK)),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.7, 0.7, 0.7))),
            text_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..base
        },
    }
}

pub fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(colors::SURFACE)),
        text_color: colors::TEXT_PRIMARY,
        border: Border {
            color: Color::from_rgb(0.8, 0.8, 0.85),
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(colors::SURFACE_DARK)),
            border: Border {
                color: colors::PRIMARY,
                ..base.border
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.9, 0.92))),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
            text_color: Color::from_rgb(0.6, 0.6, 0.6),
            ..base
        },
    }
}

pub fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(colors::ERROR)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::from_rgb(0.7, 0.1, 0.1),
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.3, 0.3))),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.7, 0.1, 0.1))),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.5, 0.5))),
            text_color: Color::from_rgb(0.9, 0.9, 0.9),
            ..base
        },
    }
}

pub fn tab_button_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let base_bg = if active {
            colors::TAB_ACTIVE
        } else {
            Color::TRANSPARENT
        };
        let text_color = if active {
            Color::WHITE
        } else {
            colors::TEXT_SECONDARY
        };

        let base = button::Style {
            background: Some(Background::Color(base_bg)),
            text_color,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 6.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(if active {
                    colors::TAB_ACTIVE
                } else {
                    colors::TAB_HOVER
                })),
                text_color: Color::WHITE,
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(Background::Color(colors::PRIMARY_DARK)),
                text_color: Color::WHITE,
                ..base
            },
            button::Status::Disabled => base,
        }
    }
}

pub fn section_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::SURFACE)),
        border: Border {
            color: Color::from_rgb(0.88, 0.88, 0.9),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(colors::TEXT_PRIMARY),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 4.0,
        },
        snap: false,
    }
}

/// Dark background for terminal-like readability.
pub fn log_viewer_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
        border: Border {
            color: Color::from_rgb(0.2, 0.2, 0.22),
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: Some(Color::from_rgb(0.9, 0.9, 0.9)),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn collapsible_header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::SURFACE_DARK)),
        border: Border {
            color: Color::from_rgb(0.85, 0.85, 0.88),
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: Some(colors::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Mutually-exclusive presets highlight the active choice prominently.
pub fn preset_button_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let base = button::Style {
            background: Some(Background::Color(if selected {
                colors::PRIMARY
            } else {
                colors::SURFACE
            })),
            text_color: if selected {
                Color::WHITE
            } else {
                colors::TEXT_PRIMARY
            },
            border: Border {
                color: if selected {
                    colors::PRIMARY_DARK
                } else {
                    Color::from_rgb(0.8, 0.8, 0.85)
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(if selected {
                    colors::PRIMARY_LIGHT
                } else {
                    colors::SURFACE_DARK
                })),
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(Background::Color(colors::PRIMARY_DARK)),
                text_color: Color::WHITE,
                ..base
            },
            button::Status::Disabled => button::Style {
                background: Some(Background::Color(Color::from_rgb(0.9, 0.9, 0.9))),
                text_color: Color::from_rgb(0.6, 0.6, 0.6),
                ..base
            },
        }
    }
}

pub fn status_indicator_color(running: bool) -> Color {
    if running {
        colors::SUCCESS
    } else {
        colors::ERROR
    }
}

/// Maps service registry QoS levels to their canonical indicator colors.
pub fn service_level_color(level: &str) -> Color {
    match level.to_lowercase().as_str() {
        "guaranteed" => colors::GUARANTEED,
        "besteffort" | "best_effort" => colors::BEST_EFFORT,
        "degraded" => colors::DEGRADED,
        "unavailable" => colors::UNAVAILABLE,
        _ => colors::TEXT_SECONDARY,
    }
}

pub fn log_level_color(level: &str) -> Color {
    match level.to_lowercase().as_str() {
        "error" => colors::LOG_ERROR,
        "warn" | "warning" => colors::LOG_WARN,
        "info" => colors::LOG_INFO,
        "debug" => colors::LOG_DEBUG,
        "trace" => colors::LOG_TRACE,
        _ => colors::TEXT_PRIMARY,
    }
}
