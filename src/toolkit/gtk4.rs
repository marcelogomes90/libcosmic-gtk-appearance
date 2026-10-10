// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_uint, c_void};

use crate::ffi::{resolve, Library};

const BLUR_PROTOCOL_MINOR: c_uint = 24;
const BLUR_PROTOCOL_PRERELEASE: (c_uint, c_uint) = (23, 3);

pub struct Symbols {
    pub gtk_owns_blur: bool,
    pub native_get_surface: extern "C" fn(*mut c_void) -> *mut c_void,
    pub surface_get_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub css_provider_load_from_string: extern "C" fn(*mut c_void, *const c_char),
    pub add_provider_for_display: extern "C" fn(*mut c_void, *mut c_void, c_uint),
}

impl Symbols {
    pub unsafe fn load(library: &Library) -> Option<Self> {
        let minor_version: extern "C" fn() -> c_uint = resolve!(library, c"gtk_get_minor_version");
        let micro_version: extern "C" fn() -> c_uint = resolve!(library, c"gtk_get_micro_version");
        let (minor, micro) = (minor_version(), micro_version());
        Some(Self {
            gtk_owns_blur: (minor >= BLUR_PROTOCOL_MINOR
                || (minor, micro) >= BLUR_PROTOCOL_PRERELEASE)
                && !crate::claimed_blur(),
            native_get_surface: resolve!(library, c"gtk_native_get_surface"),
            surface_get_display: resolve!(library, c"gdk_surface_get_display"),
            css_provider_load_from_string: resolve!(library, c"gtk_css_provider_load_from_string"),
            add_provider_for_display: resolve!(
                library,
                c"gtk_style_context_add_provider_for_display"
            ),
        })
    }
}
