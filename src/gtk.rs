use std::ffi::{c_char, c_uint, c_ulong, c_void, CStr};

use crate::ffi::{resolve, GBoolean, GQuark, GType};

#[repr(C)]
pub struct GList {
    pub data: *mut c_void,
    pub next: *mut GList,
    pub prev: *mut GList,
}

pub type SourceFunc = extern "C" fn(*mut c_void) -> GBoolean;
pub type EmissionHook = extern "C" fn(*mut c_void, c_uint, *const c_void, *mut c_void) -> GBoolean;

pub struct Gtk {
    pub type_class_ref: extern "C" fn(GType) -> *mut c_void,
    pub type_check_instance_is_a: extern "C" fn(*mut c_void, GType) -> GBoolean,
    pub signal_lookup: extern "C" fn(*const c_char, GType) -> c_uint,
    pub signal_add_emission_hook:
        extern "C" fn(c_uint, GQuark, EmissionHook, *mut c_void, *const c_void) -> c_ulong,
    pub value_get_object: extern "C" fn(*const c_void) -> *mut c_void,
    pub object_get_data: extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    pub object_set_data_full: extern "C" fn(*mut c_void, *const c_char, *mut c_void, *const c_void),
    pub object_ref: extern "C" fn(*mut c_void) -> *mut c_void,
    pub object_unref: extern "C" fn(*mut c_void),
    pub idle_add: extern "C" fn(SourceFunc, *mut c_void) -> c_uint,
    pub list_free: extern "C" fn(*mut GList),
    pub widget_get_type: extern "C" fn() -> GType,
    pub window_get_type: extern "C" fn() -> GType,
    pub widget_get_mapped: extern "C" fn(*mut c_void) -> GBoolean,
    pub widget_queue_draw: extern "C" fn(*mut c_void),
    pub window_list_toplevels: extern "C" fn() -> *mut GList,
    pub css_provider_new: extern "C" fn() -> *mut c_void,
    pub wayland_display_get_wl_display: extern "C" fn(*mut c_void) -> *mut c_void,
    pub wayland_display_get_wl_compositor: extern "C" fn(*mut c_void) -> *mut c_void,
}

impl Gtk {
    pub unsafe fn load() -> Option<Self> {
        Some(Self {
            type_class_ref: resolve!(c"g_type_class_ref"),
            type_check_instance_is_a: resolve!(c"g_type_check_instance_is_a"),
            signal_lookup: resolve!(c"g_signal_lookup"),
            signal_add_emission_hook: resolve!(c"g_signal_add_emission_hook"),
            value_get_object: resolve!(c"g_value_get_object"),
            object_get_data: resolve!(c"g_object_get_data"),
            object_set_data_full: resolve!(c"g_object_set_data_full"),
            object_ref: resolve!(c"g_object_ref"),
            object_unref: resolve!(c"g_object_unref"),
            idle_add: resolve!(c"g_idle_add"),
            list_free: resolve!(c"g_list_free"),
            widget_get_type: resolve!(c"gtk_widget_get_type"),
            window_get_type: resolve!(c"gtk_window_get_type"),
            widget_get_mapped: resolve!(c"gtk_widget_get_mapped"),
            widget_queue_draw: resolve!(c"gtk_widget_queue_draw"),
            window_list_toplevels: resolve!(c"gtk_window_list_toplevels"),
            css_provider_new: resolve!(c"gtk_css_provider_new"),
            wayland_display_get_wl_display: resolve!(c"gdk_wayland_display_get_wl_display"),
            wayland_display_get_wl_compositor: resolve!(c"gdk_wayland_display_get_wl_compositor"),
        })
    }

    pub unsafe fn widget_signal(&self, name: &CStr) -> Option<c_uint> {
        let widget = (self.widget_get_type)();
        (self.type_class_ref)(widget);
        let id = (self.signal_lookup)(name.as_ptr(), widget);
        (id != 0).then_some(id)
    }

    pub unsafe fn is_window(&self, instance: *mut c_void) -> bool {
        (self.type_check_instance_is_a)(instance, (self.window_get_type)()) != 0
    }

    pub unsafe fn is_mapped(&self, widget: *mut c_void) -> bool {
        (self.widget_get_mapped)(widget) != 0
    }
}
