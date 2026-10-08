use std::ffi::{c_char, c_uint, c_void};

use crate::ffi::{resolve, GType};

pub struct Symbols {
    pub wayland_surface_get_type: extern "C" fn() -> GType,
    pub native_get_surface: extern "C" fn(*mut c_void) -> *mut c_void,
    pub wayland_surface_get_wl_surface: extern "C" fn(*mut c_void) -> *mut c_void,
    pub surface_get_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub css_provider_load_from_string: extern "C" fn(*mut c_void, *const c_char),
    pub add_provider_for_display: extern "C" fn(*mut c_void, *mut c_void, c_uint),
}

impl Symbols {
    pub unsafe fn load() -> Option<Self> {
        Some(Self {
            wayland_surface_get_type: resolve!(c"gdk_wayland_surface_get_type"),
            native_get_surface: resolve!(c"gtk_native_get_surface"),
            wayland_surface_get_wl_surface: resolve!(c"gdk_wayland_surface_get_wl_surface"),
            surface_get_display: resolve!(c"gdk_surface_get_display"),
            css_provider_load_from_string: resolve!(c"gtk_css_provider_load_from_string"),
            add_provider_for_display: resolve!(c"gtk_style_context_add_provider_for_display"),
        })
    }
}
