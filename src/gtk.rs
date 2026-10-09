// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

pub use crate::ffi::GBoolean;
use crate::ffi::{resolve, GQuark, GType, Library};
use crate::log;

#[repr(C)]
pub struct GList {
    pub data: *mut c_void,
    pub next: *mut GList,
    pub prev: *mut GList,
}

pub type SourceFunc = extern "C" fn(*mut c_void) -> GBoolean;
pub type EmissionHook = extern "C" fn(*mut c_void, c_uint, *const c_void, *mut c_void) -> GBoolean;
pub type FdSourceFunc = extern "C" fn(c_int, c_uint, *mut c_void) -> c_int;
pub type AddFdSource =
    extern "C" fn(c_int, c_int, c_uint, FdSourceFunc, *mut c_void, *const c_void) -> c_uint;
pub type TimeoutAdd = extern "C" fn(c_uint, SourceFunc, *mut c_void) -> c_uint;

pub struct Gtk {
    pub type_check_instance_is_a: extern "C" fn(*mut c_void, GType) -> GBoolean,
    pub signal_add_emission_hook:
        extern "C" fn(c_uint, GQuark, EmissionHook, *mut c_void, *const c_void) -> c_ulong,
    pub value_get_object: extern "C" fn(*const c_void) -> *mut c_void,
    pub object_get_data: extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    pub object_set_data_full: extern "C" fn(*mut c_void, *const c_char, *mut c_void, *const c_void),
    pub object_ref: extern "C" fn(*mut c_void) -> *mut c_void,
    pub object_unref: extern "C" fn(*mut c_void),
    pub idle_add: extern "C" fn(SourceFunc, *mut c_void) -> c_uint,
    pub timeout_add: TimeoutAdd,
    pub unix_fd_add_full: AddFdSource,
    pub list_free: extern "C" fn(*mut GList),
    pub widget_get_mapped: extern "C" fn(*mut c_void) -> GBoolean,
    pub widget_queue_draw: extern "C" fn(*mut c_void),
    pub window_list_toplevels: extern "C" fn() -> *mut GList,
    pub css_provider_new: extern "C" fn() -> *mut c_void,
    pub map_signal: c_uint,
    pub window_type: GType,
}

impl Gtk {
    pub unsafe fn load(library: &Library) -> Option<Self> {
        let type_class_ref: extern "C" fn(GType) -> *mut c_void =
            resolve!(library, c"g_type_class_ref");
        let signal_lookup: extern "C" fn(*const c_char, GType) -> c_uint =
            resolve!(library, c"g_signal_lookup");
        let widget_get_type: extern "C" fn() -> GType = resolve!(library, c"gtk_widget_get_type");
        let window_get_type: extern "C" fn() -> GType = resolve!(library, c"gtk_window_get_type");

        let widget_type = widget_get_type();
        type_class_ref(widget_type);
        let map_signal = signal_lookup(c"map".as_ptr(), widget_type);
        if map_signal == 0 {
            log!("the map signal is not registered yet");
            return None;
        }

        Some(Self {
            type_check_instance_is_a: resolve!(library, c"g_type_check_instance_is_a"),
            signal_add_emission_hook: resolve!(library, c"g_signal_add_emission_hook"),
            value_get_object: resolve!(library, c"g_value_get_object"),
            object_get_data: resolve!(library, c"g_object_get_data"),
            object_set_data_full: resolve!(library, c"g_object_set_data_full"),
            object_ref: resolve!(library, c"g_object_ref"),
            object_unref: resolve!(library, c"g_object_unref"),
            idle_add: resolve!(library, c"g_idle_add"),
            timeout_add: resolve!(library, c"g_timeout_add"),
            unix_fd_add_full: resolve!(library, c"g_unix_fd_add_full"),
            list_free: resolve!(library, c"g_list_free"),
            widget_get_mapped: resolve!(library, c"gtk_widget_get_mapped"),
            widget_queue_draw: resolve!(library, c"gtk_widget_queue_draw"),
            window_list_toplevels: resolve!(library, c"gtk_window_list_toplevels"),
            css_provider_new: resolve!(library, c"gtk_css_provider_new"),
            map_signal,
            window_type: window_get_type(),
        })
    }

    pub unsafe fn is_window(&self, instance: *mut c_void) -> bool {
        (self.type_check_instance_is_a)(instance, self.window_type) != 0
    }

    pub unsafe fn is_mapped(&self, widget: *mut c_void) -> bool {
        (self.widget_get_mapped)(widget) != 0
    }
}
