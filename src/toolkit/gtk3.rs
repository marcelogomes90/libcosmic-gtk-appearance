// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_int, c_uint, c_void};

use crate::ffi::{resolve, GBoolean, Library};

pub struct Symbols {
    pub widget_get_window: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_window_type: extern "C" fn(*mut c_void) -> c_int,
    pub window_get_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_screen: extern "C" fn(*mut c_void) -> *mut c_void,
    pub css_provider_load_from_data:
        extern "C" fn(*mut c_void, *const c_char, isize, *mut *mut c_void) -> GBoolean,
    pub add_provider_for_screen: extern "C" fn(*mut c_void, *mut c_void, c_uint),
}

impl Symbols {
    pub unsafe fn load(library: &Library) -> Option<Self> {
        Some(Self {
            widget_get_window: resolve!(library, c"gtk_widget_get_window"),
            window_get_window_type: resolve!(library, c"gtk_window_get_window_type"),
            window_get_display: resolve!(library, c"gdk_window_get_display"),
            window_get_screen: resolve!(library, c"gdk_window_get_screen"),
            css_provider_load_from_data: resolve!(library, c"gtk_css_provider_load_from_data"),
            add_provider_for_screen: resolve!(
                library,
                c"gtk_style_context_add_provider_for_screen"
            ),
        })
    }
}
