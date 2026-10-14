// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

mod gtk3;
mod gtk4;

use std::ffi::{c_int, c_void, CStr};
use std::ptr;

use crate::cosmic::{Decorations, Glass};
use crate::ffi::{resolve, GType, Library};
use crate::gtk::Gtk;
use crate::log;
use crate::style;

const TOPLEVEL_WINDOW: c_int = 0;
const USER_PRIORITY: u32 = 800;
const OVERRIDE_PRIORITY: u32 = USER_PRIORITY + 1;
const SIDEBAR_CLASS: &CStr = c"sidebar";
const HEADERBAR_NODE: &CStr = c"headerbar";
const SEPARATOR_NODE: &CStr = c"separator";
const MAX_WALK_DEPTH: u32 = 12;

pub const SONAMES: [&CStr; 2] = [c"libgtk-4.so.1", c"libgtk-3.so.0"];

pub enum Toolkit {
    Gtk3(gtk3::Symbols),
    Gtk4(gtk4::Symbols),
}

impl Toolkit {
    pub unsafe fn load(library: &Library, soname: &CStr) -> Option<Self> {
        if soname == SONAMES[0] {
            gtk4::Symbols::load(library).map(Toolkit::Gtk4)
        } else {
            gtk3::Symbols::load(library).map(Toolkit::Gtk3)
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Toolkit::Gtk3(_) => "GTK3",
            Toolkit::Gtk4(_) => "GTK4",
        }
    }

    pub fn config_dir(&self) -> &'static str {
        match self {
            Toolkit::Gtk3(_) => "gtk-3.0",
            Toolkit::Gtk4(_) => "gtk-4.0",
        }
    }

    pub unsafe fn is_decorated_toplevel(&self, window: *mut c_void) -> bool {
        match self {
            Toolkit::Gtk3(s) => (s.window_get_window_type)(window) == TOPLEVEL_WINDOW,
            Toolkit::Gtk4(_) => true,
        }
    }

    pub unsafe fn surface_of(&self, window: *mut c_void) -> *mut c_void {
        match self {
            Toolkit::Gtk3(s) => (s.widget_get_window)(window),
            Toolkit::Gtk4(s) => (s.native_get_surface)(window),
        }
    }

    pub unsafe fn display_of(&self, surface: *mut c_void) -> *mut c_void {
        match self {
            Toolkit::Gtk3(s) => (s.window_get_display)(surface),
            Toolkit::Gtk4(s) => (s.surface_get_display)(surface),
        }
    }

    pub unsafe fn load_stylesheet(&self, provider: *mut c_void, css: &CStr) {
        match self {
            Toolkit::Gtk3(s) => {
                (s.css_provider_load_from_data)(provider, css.as_ptr(), -1, ptr::null_mut());
            }
            Toolkit::Gtk4(s) => (s.css_provider_load_from_string)(provider, css.as_ptr()),
        }
    }

    pub unsafe fn add_stylesheet(
        &self,
        gtk: &Gtk,
        surface: *mut c_void,
        css: &CStr,
    ) -> *mut c_void {
        let provider = (gtk.css_provider_new)();
        if provider.is_null() {
            return provider;
        }
        self.load_stylesheet(provider, css);
        match self {
            Toolkit::Gtk3(s) => {
                let screen = (s.window_get_screen)(surface);
                if !screen.is_null() {
                    (s.add_provider_for_screen)(screen, provider, OVERRIDE_PRIORITY);
                }
            }
            Toolkit::Gtk4(s) => {
                let display = (s.surface_get_display)(surface);
                if !display.is_null() {
                    (s.add_provider_for_display)(display, provider, OVERRIDE_PRIORITY);
                }
            }
        }
        provider
    }

    pub fn stylesheet(&self, glass: Option<&Glass>, decorations: Option<&Decorations>) -> String {
        match self {
            Toolkit::Gtk3(_) => style::gtk3(glass, decorations),
            Toolkit::Gtk4(s) => style::gtk4(glass, decorations, s.gtk_owns_blur),
        }
    }

    pub fn gtk_owns_blur(&self) -> bool {
        match self {
            Toolkit::Gtk3(_) => false,
            Toolkit::Gtk4(s) => s.gtk_owns_blur,
        }
    }

    pub fn fallback_palette(&self) -> &'static str {
        match self {
            Toolkit::Gtk3(_) => style::GTK3_FALLBACK_PALETTE,
            Toolkit::Gtk4(_) => "",
        }
    }

    pub unsafe fn mark_sidebar_titlebars(&self, gtk: &Gtk, window: *mut c_void) {
        let Toolkit::Gtk3(symbols) = self else {
            return;
        };
        let titlebar = (symbols.window_get_titlebar)(window);
        mark_beside_sidebars(symbols, gtk, window, titlebar, 0);
    }
}

unsafe fn is_node(symbols: &gtk3::Symbols, widget: *mut c_void, name: &CStr) -> bool {
    if widget.is_null() {
        return false;
    }
    let context = (symbols.widget_get_style_context)(widget);
    if context.is_null() {
        return false;
    }
    let path = (symbols.style_context_get_path)(context);
    if path.is_null() {
        return false;
    }
    let object = (symbols.widget_path_iter_get_object_name)(path, -1);
    !object.is_null() && CStr::from_ptr(object) == name
}

unsafe fn has_class(symbols: &gtk3::Symbols, widget: *mut c_void, class: &CStr) -> bool {
    if widget.is_null() {
        return false;
    }
    let context = (symbols.widget_get_style_context)(widget);
    !context.is_null() && (symbols.style_context_has_class)(context, class.as_ptr()) != 0
}

unsafe fn children(symbols: &gtk3::Symbols, gtk: &Gtk, widget: *mut c_void) -> Vec<*mut c_void> {
    let mut found = Vec::new();
    if !gtk.is_a(widget, symbols.container_type) {
        return found;
    }
    let list = (symbols.container_get_children)(widget);
    gtk.each_in_list(list, |child| found.push(child));
    found
}

unsafe fn mark_beside_sidebars(
    symbols: &gtk3::Symbols,
    gtk: &Gtk,
    widget: *mut c_void,
    titlebar: *mut c_void,
    depth: u32,
) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let kids = children(symbols, gtk, widget);
    for (at, divider) in kids.iter().enumerate().skip(1) {
        if !is_node(symbols, *divider, SEPARATOR_NODE)
            || !has_class(symbols, *divider, SIDEBAR_CLASS)
        {
            continue;
        }
        let before = kids[at - 1];
        let mut candidates = vec![before];
        candidates.extend(children(symbols, gtk, before));
        for candidate in candidates {
            if candidate == titlebar
                || !is_node(symbols, candidate, HEADERBAR_NODE)
                || has_class(symbols, candidate, style::SIDEBAR_TITLEBAR_CLASS)
            {
                continue;
            }
            let context = (symbols.widget_get_style_context)(candidate);
            if context.is_null() {
                continue;
            }
            (symbols.style_context_add_class)(context, style::SIDEBAR_TITLEBAR_CLASS.as_ptr());
            log!("the title bar above a sidebar now follows it");
        }
    }
    for child in kids {
        mark_beside_sidebars(symbols, gtk, child, titlebar, depth + 1);
    }
}

pub struct Wayland {
    surface_get_type: extern "C" fn() -> GType,
    surface_get_wl_surface: extern "C" fn(*mut c_void) -> *mut c_void,
    pub display_get_wl_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub display_get_wl_compositor: extern "C" fn(*mut c_void) -> *mut c_void,
}

impl Wayland {
    pub unsafe fn load(library: &Library, toolkit: &Toolkit) -> Option<Self> {
        let (get_type, get_wl_surface) = match toolkit {
            Toolkit::Gtk3(_) => (
                c"gdk_wayland_window_get_type",
                c"gdk_wayland_window_get_wl_surface",
            ),
            Toolkit::Gtk4(_) => (
                c"gdk_wayland_surface_get_type",
                c"gdk_wayland_surface_get_wl_surface",
            ),
        };
        Some(Self {
            surface_get_type: resolve!(library, get_type),
            surface_get_wl_surface: resolve!(library, get_wl_surface),
            display_get_wl_display: resolve!(library, c"gdk_wayland_display_get_wl_display"),
            display_get_wl_compositor: resolve!(library, c"gdk_wayland_display_get_wl_compositor"),
        })
    }

    pub unsafe fn wl_surface_of(&self, gtk: &Gtk, surface: *mut c_void) -> *mut c_void {
        if (gtk.type_check_instance_is_a)(surface, (self.surface_get_type)()) == 0 {
            return ptr::null_mut();
        }
        (self.surface_get_wl_surface)(surface)
    }
}
