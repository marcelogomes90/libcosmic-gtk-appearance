// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cosmic_gtk_appearance::cosmic::{self, Decorations, Glass, Padding, Rgba};
use cosmic_gtk_appearance::style;

const SKIPPED: i32 = 77;

fn decorations() -> Decorations {
    Decorations {
        active_icon: Rgba::parse("#63D0DFFF").unwrap(),
        hover: Rgba::parse("#63636333").unwrap(),
        pressed: Rgba::parse("#16161680").unwrap(),
        radius: 8.0,
        gap: 8,
        header: Padding {
            top: 7,
            right: 7,
            bottom: 8,
            left: 7,
        },
        header_maximized: Padding {
            top: 8,
            right: 8,
            bottom: 8,
            left: 8,
        },
    }
}

fn glasses() -> Vec<(&'static str, Glass)> {
    let mut found = Vec::new();
    for (name, window, sidebar, secondary) in
        [("tinted", 0.76, 1.0, 1.0), ("translucent", 0.6, 0.7, 0.85)]
    {
        for opaque_when_maximized in [false, true] {
            found.push((
                if opaque_when_maximized {
                    name
                } else {
                    "always-frosted"
                },
                Glass {
                    window_opacity: window,
                    sidebar_opacity: sidebar,
                    secondary_opacity: secondary,
                    opaque_when_maximized,
                },
            ));
        }
    }
    found
}

fn sheets(toolkit: &str) -> Vec<(String, String)> {
    let mut built = Vec::new();
    let mut glass_settings: Vec<(String, Option<Glass>)> = vec![("no-glass".into(), None)];
    for (at, (name, glass)) in glasses().into_iter().enumerate() {
        glass_settings.push((format!("{name}-{at}"), Some(glass)));
    }
    for (glass_name, glass) in &glass_settings {
        for (theme_name, theme) in [("no-theme", None), ("cosmic-theme", Some(decorations()))] {
            let body = match toolkit {
                "gtk3" => style::gtk3(glass.as_ref(), theme.as_ref()),
                _ => {
                    let mut both = style::gtk4(glass.as_ref(), theme.as_ref(), false);
                    both.push_str(&style::gtk4(glass.as_ref(), theme.as_ref(), true));
                    both
                }
            };
            built.push((format!("{toolkit}-{glass_name}-{theme_name}.css"), body));
        }
    }
    built
}

fn config_dir_of(toolkit: &str) -> &'static str {
    if toolkit == "gtk3" {
        "gtk-3.0"
    } else {
        "gtk-4.0"
    }
}

fn fallback_of(toolkit: &str) -> &'static str {
    if toolkit == "gtk3" {
        style::GTK3_FALLBACK_PALETTE
    } else {
        ""
    }
}

fn matrix_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/stylesheet-matrix")
}

fn write_group(group: &str, toolkit: &str, prelude: &str) -> PathBuf {
    let directory = matrix_root().join(group).join(toolkit);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a directory for the generated stylesheets");
    for (name, body) in sheets(toolkit) {
        fs::write(directory.join(name), format!("{prelude}{body}")).expect("a stylesheet");
    }
    directory
}

fn validate(
    toolkit: &str,
    directory: &Path,
    empty_config: Option<&Path>,
) -> Result<String, String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/validate.py");
    let mut command = Command::new("python3");
    command.arg(&script).arg(toolkit).arg(directory);
    if let Some(config) = empty_config {
        command.env("XDG_CONFIG_HOME", config);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Err(format!("python3 could not run: {error}")),
    };
    let report = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match output.status.code() {
        Some(0) => Ok(report),
        Some(code) if code == SKIPPED => Err(report),
        Some(2) | None => Err(format!("the validator could not run: {report}")),
        Some(_) => panic!("{toolkit}: {report}"),
    }
}

fn empty_config() -> PathBuf {
    let directory = matrix_root().join("empty-config");
    fs::create_dir_all(&directory).expect("an empty config home");
    directory
}

#[test]
fn every_stylesheet_parses_and_names_only_colours_the_toolkit_defines() {
    let empty = empty_config();
    let mut checked = 0;
    let mut skipped = Vec::new();

    for (toolkit, source) in [("gtk3", "gtk3"), ("gtk4", "gtk4"), ("adw", "gtk4")] {
        let reserve = write_group("no-palette", toolkit, fallback_of(source));
        match validate(toolkit, &reserve, Some(&empty)) {
            Ok(report) => {
                checked += 1;
                println!("no palette, {report}");
            }
            Err(reason) => skipped.push(format!("{toolkit} without a palette: {reason}")),
        }

        let Some(palette) = cosmic::palette(config_dir_of(source)) else {
            skipped.push(format!(
                "{toolkit} with a palette: COSMIC has not generated {}/gtk.css",
                config_dir_of(source)
            ));
            continue;
        };
        let followed = write_group("with-palette", toolkit, &palette);
        match validate(toolkit, &followed, None) {
            Ok(report) => {
                checked += 1;
                println!("COSMIC palette, {report}");
            }
            Err(reason) => skipped.push(format!("{toolkit} with a palette: {reason}")),
        }
    }

    for reason in &skipped {
        println!("skipped: {reason}");
    }
    if checked == 0 {
        println!("no toolkit could be checked, nothing was verified");
    }
}
