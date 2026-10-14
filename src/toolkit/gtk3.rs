// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_int, c_uint, c_void};

use crate::ffi::{resolve, GBoolean, GType, Library};
use crate::gtk::GList;

pub struct Symbols {
    pub widget_get_window: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_window_type: extern "C" fn(*mut c_void) -> c_int,
    pub window_get_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_screen: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_titlebar: extern "C" fn(*mut c_void) -> *mut c_void,
    pub css_provider_load_from_data:
        extern "C" fn(*mut c_void, *const c_char, isize, *mut *mut c_void) -> GBoolean,
    pub add_provider_for_screen: extern "C" fn(*mut c_void, *mut c_void, c_uint),
    pub widget_get_style_context: extern "C" fn(*mut c_void) -> *mut c_void,
    pub style_context_get_path: extern "C" fn(*mut c_void) -> *mut c_void,
    pub widget_path_iter_get_object_name: extern "C" fn(*mut c_void, c_int) -> *const c_char,
    pub style_context_has_class: extern "C" fn(*mut c_void, *const c_char) -> GBoolean,
    pub style_context_add_class: extern "C" fn(*mut c_void, *const c_char),
    pub container_get_children: extern "C" fn(*mut c_void) -> *mut GList,
    pub container_type: GType,
}

impl Symbols {
    pub unsafe fn load(library: &Library) -> Option<Self> {
        let container_get_type: extern "C" fn() -> GType =
            resolve!(library, c"gtk_container_get_type");
        Some(Self {
            widget_get_window: resolve!(library, c"gtk_widget_get_window"),
            window_get_window_type: resolve!(library, c"gtk_window_get_window_type"),
            window_get_display: resolve!(library, c"gdk_window_get_display"),
            window_get_screen: resolve!(library, c"gdk_window_get_screen"),
            window_get_titlebar: resolve!(library, c"gtk_window_get_titlebar"),
            css_provider_load_from_data: resolve!(library, c"gtk_css_provider_load_from_data"),
            add_provider_for_screen: resolve!(
                library,
                c"gtk_style_context_add_provider_for_screen"
            ),
            widget_get_style_context: resolve!(library, c"gtk_widget_get_style_context"),
            style_context_get_path: resolve!(library, c"gtk_style_context_get_path"),
            widget_path_iter_get_object_name: resolve!(
                library,
                c"gtk_widget_path_iter_get_object_name"
            ),
            style_context_has_class: resolve!(library, c"gtk_style_context_has_class"),
            style_context_add_class: resolve!(library, c"gtk_style_context_add_class"),
            container_get_children: resolve!(library, c"gtk_container_get_children"),
            container_type: container_get_type(),
        })
    }
}
