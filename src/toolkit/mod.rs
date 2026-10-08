// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

mod gtk3;
mod gtk4;

use std::ffi::{c_int, c_void, CStr};
use std::ptr;

use crate::cosmic::{Decorations, Glass};
use crate::ffi::has_symbol;
use crate::gtk::Gtk;
use crate::style;

const TOPLEVEL_WINDOW: c_int = 0;
const USER_PRIORITY: u32 = 800;
const OVERRIDE_PRIORITY: u32 = USER_PRIORITY + 1;

pub enum Toolkit {
    Gtk3(gtk3::Symbols),
    Gtk4(gtk4::Symbols),
}

impl Toolkit {
    pub unsafe fn detect() -> Option<Self> {
        if has_symbol(c"gtk_native_get_surface") {
            return gtk4::Symbols::load().map(Toolkit::Gtk4);
        }
        if has_symbol(c"gtk_widget_get_window") {
            return gtk3::Symbols::load().map(Toolkit::Gtk3);
        }
        None
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

    pub unsafe fn is_wayland(&self, gtk: &Gtk, surface: *mut c_void) -> bool {
        let wayland = match self {
            Toolkit::Gtk3(s) => (s.wayland_window_get_type)(),
            Toolkit::Gtk4(s) => (s.wayland_surface_get_type)(),
        };
        (gtk.type_check_instance_is_a)(surface, wayland) != 0
    }

    pub unsafe fn wl_surface_of(&self, surface: *mut c_void) -> *mut c_void {
        match self {
            Toolkit::Gtk3(s) => (s.wayland_window_get_wl_surface)(surface),
            Toolkit::Gtk4(s) => (s.wayland_surface_get_wl_surface)(surface),
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
        display: *mut c_void,
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
            Toolkit::Gtk4(s) => (s.add_provider_for_display)(display, provider, OVERRIDE_PRIORITY),
        }
        provider
    }

    pub fn stylesheet(&self, glass: Option<&Glass>, decorations: Option<&Decorations>) -> String {
        match self {
            Toolkit::Gtk3(_) => style::gtk3(glass, decorations),
            Toolkit::Gtk4(_) => style::gtk4(glass, decorations),
        }
    }
}
