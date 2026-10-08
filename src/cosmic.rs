// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub struct Rgba {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Rgba {
    fn parse(text: &str) -> Option<Self> {
        let hex = text.trim().trim_matches('"').trim_start_matches('#');
        if hex.len() != 8 {
            return None;
        }
        Some(Self {
            red: u8::from_str_radix(&hex[0..2], 16).ok()?,
            green: u8::from_str_radix(&hex[2..4], 16).ok()?,
            blue: u8::from_str_radix(&hex[4..6], 16).ok()?,
            alpha: u8::from_str_radix(&hex[6..8], 16).ok()?,
        })
    }
}

impl Rgba {
    pub fn opacity(self) -> f64 {
        f64::from(self.alpha) / 255.0
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "rgba({}, {}, {}, {:.3})",
            self.red,
            self.green,
            self.blue,
            f64::from(self.alpha) / 255.0
        )
    }
}

#[derive(Clone, Copy)]
pub struct Glass {
    pub window_opacity: f64,
    pub sidebar_opacity: f64,
    pub opaque_when_maximized: bool,
}

#[derive(Clone, Copy)]
pub struct Decorations {
    pub active_icon: Rgba,
    pub backdrop_icon: Rgba,
    pub hover: Rgba,
    pub pressed: Rgba,
    pub radius: f32,
}

pub struct Appearance {
    pub glass: Option<Glass>,
    pub decorations: Decorations,
}

impl Appearance {
    pub fn from_config() -> Option<Self> {
        let palette = palette_dir()?;
        let read = |name: &str| std::fs::read_to_string(palette.join(name)).ok();

        let icon_button = read("icon_button")?;
        let accent = read("accent")?;
        let corner_radii = read("corner_radii")?;

        let decorations = Decorations {
            active_icon: Rgba::parse(field(&accent, "base")?)?,
            backdrop_icon: Rgba::parse(field(&icon_button, "on")?)?,
            hover: Rgba::parse(field(&icon_button, "hover")?)?,
            pressed: Rgba::parse(field(&icon_button, "pressed")?)?,
            radius: first_number(field(&corner_radii, "radius_s")?)?,
        };

        let glass = read("frosted_windows")
            .filter(|flag| flag.trim() == "true")
            .and_then(|_| {
                let background = read("transparent_background")?;
                let primary = read("transparent_primary")?;
                Some(Glass {
                    window_opacity: Rgba::parse(field(&background, "base")?)?.opacity(),
                    sidebar_opacity: Rgba::parse(field(&primary, "base")?)?.opacity(),
                    opaque_when_maximized: read("frosted_maximized_apps")
                        .is_some_and(|flag| flag.trim() != "true"),
                })
            });

        Some(Self { glass, decorations })
    }
}

pub fn watched_directories() -> Vec<PathBuf> {
    let Some(root) = config_root() else {
        return Vec::new();
    };
    [
        "com.system76.CosmicTheme.Mode/v1",
        "com.system76.CosmicTheme.Dark/v2",
        "com.system76.CosmicTheme.Light/v2",
    ]
    .iter()
    .map(|leaf| root.join(leaf))
    .filter(|path| path.is_dir())
    .collect()
}

fn config_root() -> Option<PathBuf> {
    let roots = [
        std::env::var_os("XDG_CONFIG_HOME").map(|base| PathBuf::from(base).join("cosmic")),
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/cosmic")),
    ];
    roots.into_iter().flatten().find(|path| {
        path.join("com.system76.CosmicTheme.Mode/v1/is_dark")
            .exists()
    })
}

fn palette_dir() -> Option<PathBuf> {
    let root = config_root()?;
    let dark = std::fs::read_to_string(root.join("com.system76.CosmicTheme.Mode/v1/is_dark"))
        .map_or(true, |value| value.trim() != "false");
    Some(
        root.join(if dark {
            "com.system76.CosmicTheme.Dark"
        } else {
            "com.system76.CosmicTheme.Light"
        })
        .join("v2"),
    )
}

fn field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix(key)?.strip_prefix(':')?;
        Some(value.trim().trim_end_matches(','))
    })
}

fn first_number(text: &str) -> Option<f32> {
    text.trim_matches(|c| c == '(' || c == ')')
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{field, first_number, Rgba};

    const ICON_BUTTON: &str = r##"(
    base: "#00000000",
    hover: "#63636333",
    pressed: "#16161680",
    on: "#BEBEBEFF",
    on_disabled: "#BEBEBEA6",
)"##;

    const CORNER_RADII: &str = r"(
    radius_0: (0.0, 0.0, 0.0, 0.0),
    radius_s: (2.0, 2.0, 2.0, 2.0),
)";

    #[test]
    fn converts_hex_with_alpha_to_css() {
        let hover = Rgba::parse(field(ICON_BUTTON, "hover").unwrap()).unwrap();
        assert_eq!(hover.to_string(), "rgba(99, 99, 99, 0.200)");
    }

    #[test]
    fn opaque_colour_keeps_full_alpha() {
        let icon = Rgba::parse(field(ICON_BUTTON, "on").unwrap()).unwrap();
        assert_eq!(icon.to_string(), "rgba(190, 190, 190, 1.000)");
    }

    #[test]
    fn longer_key_does_not_shadow_shorter_one() {
        assert_eq!(field(ICON_BUTTON, "on"), Some("\"#BEBEBEFF\""));
    }

    #[test]
    fn missing_key_is_none() {
        assert_eq!(field(ICON_BUTTON, "accent"), None);
    }

    #[test]
    fn radius_takes_the_first_corner() {
        let radius = first_number(field(CORNER_RADII, "radius_s").unwrap());
        assert_eq!(radius, Some(2.0));
    }

    #[test]
    fn rejects_malformed_colour() {
        assert!(Rgba::parse("\"#FFF\"").is_none());
    }
}
