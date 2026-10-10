// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::fmt::Write;

use crate::cosmic::{Decorations, Glass, Padding};

const PLAIN_ICON: &str = concat!(
    "button.image-button:not(.titlebutton):not(.suggested-action)",
    ":not(.destructive-action):not(.opaque):not(.raised)"
);
const CONTROL_ICON: u16 = 16;
const CONTROL_PADDING: u16 = 8;
const HEADER_CONTENT: u16 = 32;

struct Tokens {
    active_icon: String,
    backdrop_icon: String,
    hover: String,
    pressed: String,
    radius: String,
}

impl Tokens {
    fn resolve(decorations: Option<&Decorations>) -> Self {
        match decorations {
            Some(theme) => Self {
                active_icon: theme.active_icon.to_string(),
                backdrop_icon: theme.backdrop_icon.to_string(),
                hover: theme.hover.to_string(),
                pressed: theme.pressed.to_string(),
                radius: format!("{}px", theme.radius),
            },
            None => Self {
                active_icon: "@headerbar_fg_color".into(),
                backdrop_icon: "@headerbar_fg_color".into(),
                hover: "alpha(@headerbar_fg_color, 0.15)".into(),
                pressed: "alpha(@headerbar_fg_color, 0.28)".into(),
                radius: "4px".into(),
            },
        }
    }
}

fn title(sheet: &mut String, backdrop_icon: &str) {
    let _ = write!(
        sheet,
        "headerbar .title, headerbar .subtitle, .titlebar .title, .titlebar .subtitle {{
  color: @headerbar_fg_color;
}}

headerbar .title:backdrop, headerbar .subtitle:backdrop,
.titlebar .title:backdrop, .titlebar .subtitle:backdrop {{
  color: {backdrop_icon};
}}
"
    );
}

fn control(padding: &Padding) -> (u16, String) {
    let Padding {
        top,
        right,
        bottom,
        left,
    } = *padding;
    (
        HEADER_CONTENT + top + bottom,
        format!("{top}px {right}px {bottom}px {left}px"),
    )
}

pub fn gtk4(
    glass: Option<&Glass>,
    decorations: Option<&Decorations>,
    gtk_owns_blur: bool,
) -> String {
    let mut sheet = String::new();
    if let Some(glass) = glass {
        let backdrop = if gtk_owns_blur {
            "\n  backdrop-filter: blur(32px);"
        } else {
            ""
        };
        let _ = write!(
            sheet,
            "window.background, window.csd, dialog.background {{
  background-color: alpha(@window_bg_color, {window});{backdrop}
}}

.sidebar-pane, .navigation-sidebar, .sidebar {{
  background-color: alpha(@sidebar_bg_color, {sidebar});
}}

textview, textview > text, picture {{
  background-color: @view_bg_color;
}}

popovermenubar, popovermenubar:backdrop {{
  background-color: @window_bg_color;
}}

",
            window = glass.window_opacity,
            sidebar = glass.sidebar_opacity
        );
        if glass.opaque_when_maximized {
            sheet.push_str(
                "window.background.maximized, window.csd.maximized,
window.background.fullscreen, window.csd.fullscreen,
dialog.background.maximized, dialog.background.fullscreen {
  background-color: @window_bg_color;
}

window.maximized headerbar, window.maximized .titlebar,
window.fullscreen headerbar, window.fullscreen .titlebar {
  background-color: @headerbar_bg_color;
}

window.maximized .sidebar-pane, window.maximized .navigation-sidebar,
window.maximized .sidebar, window.fullscreen .sidebar-pane,
window.fullscreen .navigation-sidebar, window.fullscreen .sidebar {
  background-color: @sidebar_bg_color;
}

",
            );
        }
    }

    let Tokens {
        active_icon,
        backdrop_icon,
        hover,
        pressed,
        radius,
    } = Tokens::resolve(decorations);

    let _ = write!(
        sheet,
        "headerbar, headerbar:backdrop, headerbar > windowhandle, windowhandle, .titlebar,
toolbarview > .top-bar, toolbarview > .top-bar.raised,
toolbarview > .bottom-bar, toolbarview > .bottom-bar.raised {{
  background: none;
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border-color: transparent;
  color: @headerbar_fg_color;
  transition: none;
}}

headerbar button.titlebutton, windowcontrols > button {{
  background: none;
  background-image: none;
  box-shadow: none;
  border: none;
  outline: none;
  margin: 0;
  padding: 0;
  min-width: {chip}px;
  min-height: {chip}px;
  transition: none;
}}

windowcontrols > button > image {{
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border-radius: {radius};
  min-width: {icon}px;
  min-height: {icon}px;
  padding: {inset}px;
  transition: none;
}}

windowcontrols > button:hover > image {{
  background-color: {hover};
}}

windowcontrols > button:active > image,
windowcontrols > button:checked > image {{
  background-color: {pressed};
}}

windowcontrols > button, windowcontrols > button > image {{
  color: {active_icon};
}}

windowcontrols > button:backdrop, windowcontrols > button:backdrop > image,
windowcontrols > button > image:backdrop {{
  color: {backdrop_icon};
}}

headerbar {plain}, .titlebar {plain} {{
  color: {active_icon};
}}

headerbar {plain}:backdrop, .titlebar {plain}:backdrop {{
  color: {backdrop_icon};
}}

",
        plain = PLAIN_ICON,
        chip = CONTROL_ICON + 2 * CONTROL_PADDING,
        icon = CONTROL_ICON,
        inset = CONTROL_PADDING,
    );

    if let Some(theme) = decorations {
        let (height, padding) = control(&theme.header);
        let (maximized_height, maximized_padding) = control(&theme.header_maximized);
        let _ = write!(
            sheet,
            "headerbar, .titlebar {{
  min-height: {height}px;
}}

headerbar > windowhandle > box {{
  padding: {padding};
}}

window.maximized headerbar, window.maximized .titlebar,
window.fullscreen headerbar, window.fullscreen .titlebar {{
  min-height: {maximized_height}px;
}}

window.maximized headerbar > windowhandle > box,
window.fullscreen headerbar > windowhandle > box {{
  padding: {maximized_padding};
}}

windowcontrols {{
  border-spacing: {gap}px;
}}

",
            gap = theme.gap
        );
    }
    title(&mut sheet, &backdrop_icon);
    sheet
}

pub fn gtk3(glass: Option<&Glass>, decorations: Option<&Decorations>) -> String {
    let mut sheet = String::new();
    if let Some(glass) = glass {
        let _ = write!(
            sheet,
            "window:not(.popup) decoration, dialog:not(.popup) decoration,
messagedialog:not(.popup) decoration {{
  background-color: alpha(@window_bg_color, {window});
}}

window.background:not(.popup), dialog.background:not(.popup),
messagedialog.background:not(.popup) {{
  background-color: transparent;
}}

.sidebar, placessidebar, placessidebar list {{
  background-color: alpha(@sidebar_bg_color, {sidebar});
}}

textview, textview text {{
  background-color: @view_bg_color;
}}

menubar, menubar:backdrop, .menubar, .menubar:backdrop {{
  background-color: @window_bg_color;
}}

",
            window = glass.window_opacity,
            sidebar = glass.sidebar_opacity
        );
        if glass.opaque_when_maximized {
            sheet.push_str(
                "window:not(.popup).maximized.background,
window:not(.popup).fullscreen.background,
dialog:not(.popup).maximized.background,
dialog:not(.popup).fullscreen.background,
window:not(.popup).maximized decoration,
window:not(.popup).fullscreen decoration {
  background-color: @window_bg_color;
}

window:not(.popup).maximized headerbar, window:not(.popup).maximized .titlebar,
window:not(.popup).fullscreen headerbar, window:not(.popup).fullscreen .titlebar,
dialog:not(.popup).maximized headerbar, dialog:not(.popup).maximized .titlebar {
  background-color: @headerbar_bg_color;
}

.maximized .sidebar, .maximized placessidebar,
.fullscreen .sidebar, .fullscreen placessidebar {
  background-color: @sidebar_bg_color;
}

",
            );
        }
    }

    let Tokens {
        active_icon,
        backdrop_icon,
        hover,
        pressed,
        radius,
    } = Tokens::resolve(decorations);

    let _ = write!(
        sheet,
        "headerbar, headerbar:backdrop, .titlebar, .header-bar {{
  background: none;
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border-color: transparent;
  color: @headerbar_fg_color;
  transition: none;
}}

headerbar button.titlebutton, .titlebar button.titlebutton {{
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border: none;
  outline: none;
  border-radius: {radius};
  margin: 0;
  min-width: {icon}px;
  min-height: {icon}px;
  padding: {inset}px;
  transition: none;
}}

headerbar button.titlebutton:hover, .titlebar button.titlebutton:hover {{
  background-color: {hover};
}}

headerbar button.titlebutton:active, .titlebar button.titlebutton:active {{
  background-color: {pressed};
}}

headerbar button.titlebutton, .titlebar button.titlebutton,
headerbar button.titlebutton image, .titlebar button.titlebutton image {{
  color: {active_icon};
}}

headerbar button.titlebutton:backdrop, .titlebar button.titlebutton:backdrop,
headerbar button.titlebutton:backdrop image,
.titlebar button.titlebutton:backdrop image {{
  color: {backdrop_icon};
}}

headerbar {plain}, .titlebar {plain},
headerbar {plain} image, .titlebar {plain} image {{
  color: {active_icon};
}}

headerbar {plain}:backdrop, .titlebar {plain}:backdrop,
headerbar {plain}:backdrop image, .titlebar {plain}:backdrop image {{
  color: {backdrop_icon};
}}

",
        plain = PLAIN_ICON,
        icon = CONTROL_ICON,
        inset = CONTROL_PADDING,
    );

    if let Some(theme) = decorations {
        let (_, padding) = control(&theme.header);
        let (_, maximized_padding) = control(&theme.header_maximized);
        let _ = write!(
            sheet,
            "headerbar, .titlebar {{
  min-height: {content}px;
  padding: {padding};
}}

.maximized headerbar, .maximized .titlebar,
.fullscreen headerbar, .fullscreen .titlebar {{
  padding: {maximized_padding};
}}

",
            content = HEADER_CONTENT
        );
    }
    title(&mut sheet, &backdrop_icon);
    sheet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmic::Rgba;

    fn decorations() -> Decorations {
        Decorations {
            active_icon: Rgba::parse("#63D0DFFF").unwrap(),
            backdrop_icon: Rgba::parse("#BEBEBEFF").unwrap(),
            hover: Rgba::parse("#63636333").unwrap(),
            pressed: Rgba::parse("#16161680").unwrap(),
            radius: 160.0,
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

    fn glass() -> Glass {
        Glass {
            window_opacity: 0.76,
            sidebar_opacity: 1.0,
            opaque_when_maximized: true,
        }
    }

    #[test]
    fn without_glass_no_surface_is_made_translucent() {
        for sheet in [gtk3(None, None), gtk4(None, None, false)] {
            assert!(!sheet.contains("alpha(@window_bg_color"));
            assert!(!sheet.contains("background-color: transparent;\n}"));
        }
    }

    #[test]
    fn with_glass_the_window_background_is_tinted() {
        assert!(gtk3(Some(&glass()), None).contains("alpha(@window_bg_color, 0.76)"));
        assert!(gtk4(Some(&glass()), None, false).contains("alpha(@window_bg_color, 0.76)"));
    }

    #[test]
    fn gtk3_tints_the_decoration_node_not_the_window() {
        let sheet = gtk3(Some(&glass()), None);
        assert!(sheet.contains("window:not(.popup) decoration"));
        assert!(sheet.contains("dialog:not(.popup) decoration"));
    }

    #[test]
    fn popups_are_excluded_from_the_glass() {
        let sheet = gtk3(Some(&glass()), None);
        for rule in sheet.split('}') {
            if rule.contains("alpha(@window_bg_color") {
                assert!(rule.contains(":not(.popup)"));
            }
        }
    }

    #[test]
    fn maximized_override_reaches_the_title_bar() {
        for sheet in [
            gtk3(Some(&glass()), None),
            gtk4(Some(&glass()), None, false),
        ] {
            assert!(sheet.contains(".maximized headerbar"));
            assert!(sheet.contains(".maximized .titlebar"));
        }
    }

    #[test]
    fn pictures_stay_solid_so_the_desktop_does_not_show_through_them() {
        assert!(gtk4(Some(&glass()), None, false).contains("picture"));
        assert!(!gtk4(None, None, false).contains("picture"));
    }

    #[test]
    fn text_views_stay_solid() {
        for sheet in [
            gtk3(Some(&glass()), None),
            gtk4(Some(&glass()), None, false),
        ] {
            assert!(sheet.contains("background-color: @view_bg_color;"));
        }
    }

    #[test]
    fn a_gtk_that_speaks_the_protocol_is_asked_through_css() {
        let native = gtk4(Some(&glass()), None, true);
        assert!(native.contains("backdrop-filter: blur("));
        assert!(!gtk4(Some(&glass()), None, false).contains("backdrop-filter"));
    }

    #[test]
    fn no_glass_means_no_backdrop_filter() {
        assert!(!gtk4(None, None, true).contains("backdrop-filter"));
    }

    #[test]
    fn the_accent_reaches_icons_and_never_a_filled_button() {
        let theme = decorations();
        let accent = theme.active_icon.to_string();
        for sheet in [gtk3(None, Some(&theme)), gtk4(None, Some(&theme), false)] {
            for rule in sheet.split('}').filter(|rule| rule.contains(&accent)) {
                assert!(
                    rule.contains("windowcontrols")
                        || rule.contains("button.titlebutton")
                        || rule.contains("button.image-button")
                );
                if rule.contains("image-button") {
                    for filled in [
                        ".suggested-action",
                        ".destructive-action",
                        ".opaque",
                        ".raised",
                    ] {
                        assert!(rule.contains(&format!(":not({filled})")));
                    }
                }
            }
        }
    }

    #[test]
    fn an_icon_button_in_the_title_bar_takes_the_accent() {
        let theme = decorations();
        for sheet in [gtk3(None, Some(&theme)), gtk4(None, Some(&theme), false)] {
            assert!(sheet.contains(&format!("headerbar {PLAIN_ICON},")));
        }
    }

    #[test]
    fn a_window_control_is_an_icon_inside_the_cosmic_radius() {
        let theme = decorations();
        for sheet in [gtk3(None, Some(&theme)), gtk4(None, Some(&theme), false)] {
            assert!(sheet.contains("border-radius: 160px;"));
            assert!(sheet.contains("padding: 8px;"));
            assert!(sheet.contains("min-width: 16px;"));
        }
    }

    #[test]
    fn the_title_bar_takes_its_height_from_the_density() {
        let sheet = gtk4(None, Some(&decorations()), false);
        assert!(sheet.contains("min-height: 47px;"));
        assert!(sheet.contains("padding: 7px 7px 8px 7px;"));
        assert!(sheet.contains("min-height: 48px;"));
        assert!(sheet.contains("padding: 8px 8px 8px 8px;"));
        assert!(sheet.contains("border-spacing: 8px;"));
    }

    #[test]
    fn without_a_cosmic_theme_the_metrics_are_left_alone() {
        let sheet = gtk4(None, None, false);
        assert!(!sheet.contains("border-spacing"));
        assert!(!sheet.contains("headerbar > windowhandle > box"));
    }
}
