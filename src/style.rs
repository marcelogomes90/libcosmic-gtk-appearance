// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use crate::cosmic::{Decorations, Glass};

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

pub fn gtk4(glass: Option<&Glass>, decorations: Option<&Decorations>) -> String {
    let mut sheet = String::new();
    if let Some(glass) = glass {
        sheet.push_str(&format!(
            "window.background, window.csd, dialog.background {{
  background-color: alpha(@window_bg_color, {window});
}}

.sidebar-pane, .navigation-sidebar, .sidebar {{
  background-color: alpha(@sidebar_bg_color, {sidebar});
}}

textview, textview > text {{
  background-color: @view_bg_color;
}}

popovermenubar, popovermenubar:backdrop {{
  background-color: @window_bg_color;
}}

",
            window = glass.window_opacity,
            sidebar = glass.sidebar_opacity
        ));
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

    sheet.push_str(&format!(
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
  transition: none;
}}

windowcontrols > button > image {{
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border-radius: {radius};
  padding: 5px;
  transition: none;
}}

windowcontrols > button:hover > image {{
  background-color: {hover};
}}

windowcontrols > button:active > image,
windowcontrols > button:checked > image {{
  background-color: {pressed};
}}

headerbar button, headerbar menubutton, headerbar menubutton > button,
windowcontrols > button, windowcontrols > button > image, headerbar button image {{
  color: {active_icon};
}}

headerbar button:backdrop, headerbar menubutton:backdrop,
headerbar menubutton > button:backdrop, windowcontrols > button:backdrop,
windowcontrols > button:backdrop > image, windowcontrols > button > image:backdrop,
headerbar button:backdrop image, headerbar button image:backdrop {{
  color: {backdrop_icon};
}}

headerbar label, headerbar button label, .titlebar label {{
  color: @headerbar_fg_color;
}}

headerbar label:backdrop, headerbar button label:backdrop, .titlebar label:backdrop {{
  color: {backdrop_icon};
}}
"
    ));
    sheet
}

pub fn gtk3(glass: Option<&Glass>, decorations: Option<&Decorations>) -> String {
    let mut sheet = String::new();
    if let Some(glass) = glass {
        sheet.push_str(&format!(
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
        ));
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

    sheet.push_str(&format!(
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
  padding: 5px;
  transition: none;
}}

headerbar button.titlebutton:hover, .titlebar button.titlebutton:hover {{
  background-color: {hover};
}}

headerbar button.titlebutton:active, .titlebar button.titlebutton:active {{
  background-color: {pressed};
}}

headerbar button, headerbar menubutton, headerbar button.titlebutton,
.titlebar button, .titlebar button.titlebutton,
headerbar button image, .titlebar button image {{
  color: {active_icon};
}}

headerbar button:backdrop, headerbar menubutton:backdrop,
headerbar button.titlebutton:backdrop, .titlebar button:backdrop,
.titlebar button.titlebutton:backdrop,
headerbar button:backdrop image, .titlebar button:backdrop image {{
  color: {backdrop_icon};
}}

headerbar label, headerbar button label, .titlebar label {{
  color: @headerbar_fg_color;
}}

headerbar label:backdrop, headerbar button label:backdrop, .titlebar label:backdrop {{
  color: {backdrop_icon};
}}
"
    ));
    sheet
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glass() -> Glass {
        Glass {
            window_opacity: 0.76,
            sidebar_opacity: 1.0,
            opaque_when_maximized: true,
        }
    }

    #[test]
    fn without_glass_no_surface_is_made_translucent() {
        for sheet in [gtk3(None, None), gtk4(None, None)] {
            assert!(!sheet.contains("alpha(@window_bg_color"));
            assert!(!sheet.contains("background-color: transparent;\n}"));
        }
    }

    #[test]
    fn with_glass_the_window_background_is_tinted() {
        assert!(gtk3(Some(&glass()), None).contains("alpha(@window_bg_color, 0.76)"));
        assert!(gtk4(Some(&glass()), None).contains("alpha(@window_bg_color, 0.76)"));
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
        for sheet in [gtk3(Some(&glass()), None), gtk4(Some(&glass()), None)] {
            assert!(sheet.contains(".maximized headerbar"));
            assert!(sheet.contains(".maximized .titlebar"));
        }
    }

    #[test]
    fn text_views_stay_solid() {
        for sheet in [gtk3(Some(&glass()), None), gtk4(Some(&glass()), None)] {
            assert!(sheet.contains("background-color: @view_bg_color;"));
        }
    }
}
