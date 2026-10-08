use std::ffi::{c_char, c_int, c_uint, c_void};

use crate::ffi::{resolve, GBoolean, GType};

pub struct Symbols {
    pub wayland_window_get_type: extern "C" fn() -> GType,
    pub widget_get_window: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_window_type: extern "C" fn(*mut c_void) -> c_int,
    pub wayland_window_get_wl_surface: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub window_get_screen: extern "C" fn(*mut c_void) -> *mut c_void,
    pub css_provider_load_from_data:
        extern "C" fn(*mut c_void, *const c_char, isize, *mut *mut c_void) -> GBoolean,
    pub add_provider_for_screen: extern "C" fn(*mut c_void, *mut c_void, c_uint),
}

impl Symbols {
    pub unsafe fn load() -> Option<Self> {
        Some(Self {
            wayland_window_get_type: resolve!(c"gdk_wayland_window_get_type"),
            widget_get_window: resolve!(c"gtk_widget_get_window"),
            window_get_window_type: resolve!(c"gtk_window_get_window_type"),
            wayland_window_get_wl_surface: resolve!(c"gdk_wayland_window_get_wl_surface"),
            window_get_display: resolve!(c"gdk_window_get_display"),
            window_get_screen: resolve!(c"gdk_window_get_screen"),
            css_provider_load_from_data: resolve!(c"gtk_css_provider_load_from_data"),
            add_provider_for_screen: resolve!(c"gtk_style_context_add_provider_for_screen"),
        })
    }
}
