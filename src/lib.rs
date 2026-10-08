// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

mod cosmic;
mod ffi;
mod gtk;
mod style;
mod toolkit;
mod wayland;

use std::cell::UnsafeCell;
use std::ffi::{c_uint, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::OnceLock;

use cosmic::{Appearance, Decorations, Glass};
use ffi::GBoolean;
use gtk::{Gtk, SourceFunc};
use toolkit::Toolkit;
use wayland::{BlurEffect, BlurManager};

macro_rules! log {
    ($($arg:tt)*) => {
        if $crate::logging_enabled() {
            eprintln!("[cosmic-gtk-appearance] {}", format_args!($($arg)*));
        }
    };
}

pub(crate) use log;

pub(crate) fn logging_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("COSMIC_GTK_APPEARANCE_DEBUG").is_some())
}

const MIN_OPACITY: f64 = 0.05;
const WINDOW_DATA_KEY: &CStr = c"libcosmic-gtk-appearance";
const SOURCE_REMOVE: GBoolean = 0;
const KEEP_EMISSION_HOOK: GBoolean = 1;
const BLUR_EXTENT: i32 = 32767;

struct Session {
    gtk: Gtk,
    toolkit: Toolkit,
    compositor: *mut c_void,
    blur: Option<BlurManager>,
    decorations: Option<Decorations>,
    glass: Option<Glass>,
    searched_for_blur: bool,
    stylesheet_installed: bool,
}

impl Session {
    unsafe fn connect_blur(&mut self, display: *mut c_void) -> Option<()> {
        if !self.searched_for_blur {
            self.searched_for_blur = true;
            let wl_display = (self.gtk.wayland_display_get_wl_display)(display);
            let compositor = (self.gtk.wayland_display_get_wl_compositor)(display);
            if wl_display.is_null() || compositor.is_null() {
                log!("no wl_display or wl_compositor");
                return None;
            }
            self.compositor = compositor;
            self.blur = BlurManager::bind(wl_display);
        }
        self.blur.as_ref().map(|_| ())
    }

    unsafe fn install_stylesheet(
        &mut self,
        surface: *mut c_void,
        display: *mut c_void,
        frosted: bool,
    ) {
        if self.stylesheet_installed {
            return;
        }
        self.stylesheet_installed = true;
        if std::env::var_os("COSMIC_GTK_APPEARANCE_NO_CSS").is_some() {
            return;
        }
        let Some(css) = self.compose_stylesheet(frosted) else {
            return;
        };
        self.toolkit
            .add_stylesheet(&self.gtk, surface, display, &css);
        log!(
            "stylesheet applied ({}, frosted glass {})",
            self.toolkit.name(),
            if frosted { "on" } else { "off" }
        );
    }

    fn compose_stylesheet(&self, frosted: bool) -> Option<CString> {
        let glass = if frosted { self.glass } else { None };
        let mut css = read_env_file("COSMIC_GTK_APPEARANCE_CSS").unwrap_or_else(|| {
            self.toolkit
                .stylesheet(glass.as_ref(), self.decorations.as_ref())
        });
        if let Some(extra) = read_env_file("COSMIC_GTK_APPEARANCE_CSS_EXTRA") {
            css.push('\n');
            css.push_str(&extra);
        }
        CString::new(css).ok()
    }
}

struct SessionCell(UnsafeCell<Option<Session>>);

unsafe impl Sync for SessionCell {}

static SESSION: SessionCell = SessionCell(UnsafeCell::new(None));

unsafe fn session_ref() -> Option<&'static Session> {
    (*SESSION.0.get()).as_ref()
}

unsafe fn session_mut() -> Option<&'static mut Session> {
    (*SESSION.0.get()).as_mut()
}

struct WindowBlur {
    effect: Option<BlurEffect>,
    wl_surface: *mut c_void,
}

extern "C" fn release_window_blur(data: *mut c_void) {
    if !data.is_null() {
        unsafe { drop(Box::from_raw(data as *mut WindowBlur)) }
    }
}

#[used]
#[link_section = ".init_array"]
static INITIALIZER: extern "C" fn() = on_library_loaded;

extern "C" fn on_library_loaded() {
    let _ = catch_unwind(|| unsafe { schedule_start() });
}

unsafe fn schedule_start() {
    if std::env::var_os("COSMIC_GTK_APPEARANCE_DISABLE").is_some() {
        return;
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return;
    }
    let Some(address) = ffi::symbol(c"g_idle_add") else {
        return;
    };
    let idle_add: extern "C" fn(SourceFunc, *mut c_void) -> c_uint = std::mem::transmute(address);
    idle_add(start, ptr::null_mut());
}

extern "C" fn start(_data: *mut c_void) -> GBoolean {
    let _ = catch_unwind(|| unsafe {
        start_session();
    });
    SOURCE_REMOVE
}

unsafe fn start_session() -> Option<()> {
    if session_ref().is_some() {
        return Some(());
    }

    let gtk = Gtk::load()?;
    let toolkit = Toolkit::detect()?;
    let map_signal = gtk.widget_signal(c"map")?;
    let appearance = Appearance::from_config();
    let decorations = appearance.as_ref().map(|found| found.decorations);
    let glass = resolve_glass(appearance.as_ref());
    log!(
        "toolkit {}, map signal {map_signal}, COSMIC theme {}, frosted glass {}",
        toolkit.name(),
        if appearance.is_some() {
            "loaded"
        } else {
            "not found, falling back to GTK colours"
        },
        if glass.is_some() { "on" } else { "off" }
    );

    let add_emission_hook = gtk.signal_add_emission_hook;
    let list_toplevels = gtk.window_list_toplevels;
    let list_free = gtk.list_free;

    *SESSION.0.get() = Some(Session {
        gtk,
        toolkit,
        compositor: ptr::null_mut(),
        blur: None,
        decorations,
        glass,
        searched_for_blur: false,
        stylesheet_installed: false,
    });

    add_emission_hook(
        map_signal,
        0,
        on_window_mapped,
        ptr::null_mut(),
        ptr::null(),
    );

    let toplevels = list_toplevels();
    let mut node = toplevels;
    while !node.is_null() {
        defer_apply((*node).data);
        node = (*node).next;
    }
    if !toplevels.is_null() {
        list_free(toplevels);
    }
    Some(())
}

extern "C" fn on_window_mapped(
    _hint: *mut c_void,
    parameter_count: c_uint,
    parameters: *const c_void,
    _data: *mut c_void,
) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        if parameter_count < 1 {
            return;
        }
        let Some(session) = session_ref() else {
            return;
        };
        let instance = (session.gtk.value_get_object)(parameters);
        if instance.is_null() || !session.gtk.is_window(instance) {
            return;
        }
        defer_apply(instance);
    }));
    KEEP_EMISSION_HOOK
}

unsafe fn defer_apply(window: *mut c_void) {
    if window.is_null() {
        return;
    }
    let Some(session) = session_ref() else {
        return;
    };
    (session.gtk.object_ref)(window);
    (session.gtk.idle_add)(apply_to_window, window);
}

extern "C" fn apply_to_window(window: *mut c_void) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        attach_blur(window);
        if let Some(session) = session_ref() {
            (session.gtk.object_unref)(window);
        }
    }));
    SOURCE_REMOVE
}

unsafe fn attach_blur(window: *mut c_void) -> Option<()> {
    let session = session_mut()?;
    if !session.gtk.is_mapped(window) {
        return None;
    }

    if !session.toolkit.is_decorated_toplevel(window) {
        log!("skipping popup window {window:p}");
        return None;
    }

    let surface = session.toolkit.surface_of(window);
    if surface.is_null() {
        return None;
    }
    if !session.toolkit.is_wayland(&session.gtk, surface) {
        log!("surface does not come from the wayland backend");
        return None;
    }

    let wl_surface = session.toolkit.wl_surface_of(surface);
    let display = session.toolkit.display_of(surface);
    if wl_surface.is_null() || display.is_null() {
        return None;
    }

    let frosted =
        session.glass.is_some() && session.ensure_blur(window, surface, wl_surface, display);
    session.install_stylesheet(surface, display, frosted);
    Some(())
}

impl Session {
    unsafe fn ensure_blur(
        &mut self,
        window: *mut c_void,
        surface: *mut c_void,
        wl_surface: *mut c_void,
        display: *mut c_void,
    ) -> bool {
        if self.connect_blur(display).is_none() {
            return false;
        }
        let Some(manager) = self.blur.as_ref() else {
            return false;
        };
        if !manager.supports_blur() {
            log!(
                "compositor lacks the blur capability (caps=0x{:x})",
                manager.capabilities()
            );
            return false;
        }

        let mut stored =
            (self.gtk.object_get_data)(surface, WINDOW_DATA_KEY.as_ptr()) as *mut WindowBlur;
        if stored.is_null() {
            stored = Box::into_raw(Box::new(WindowBlur {
                effect: None,
                wl_surface: ptr::null_mut(),
            }));
            (self.gtk.object_set_data_full)(
                surface,
                WINDOW_DATA_KEY.as_ptr(),
                stored.cast(),
                release_window_blur as *const c_void,
            );
        }
        let record = &mut *stored;

        if record.wl_surface != wl_surface {
            if let Some(stale) = record.effect.take() {
                stale.destroy();
            }
            let Some(manager) = self.blur.as_ref() else {
                return false;
            };
            let Some(effect) = manager.effect_for(wl_surface) else {
                log!("get_background_effect failed");
                return false;
            };
            effect.set_region(self.compositor, BLUR_EXTENT);
            record.effect = Some(effect);
            record.wl_surface = wl_surface;
            (self.gtk.widget_queue_draw)(window);
            log!("blur attached to window {window:p}");
        }
        true
    }
}

fn resolve_glass(appearance: Option<&Appearance>) -> Option<Glass> {
    let configured = appearance.and_then(|found| found.glass);
    let Some(forced) = forced_opacity() else {
        return configured;
    };
    Some(Glass {
        window_opacity: forced,
        sidebar_opacity: forced,
        opaque_when_maximized: configured.is_some_and(|glass| glass.opaque_when_maximized),
    })
}

fn forced_opacity() -> Option<f64> {
    let value: f64 = std::env::var("COSMIC_GTK_APPEARANCE_ALPHA")
        .ok()?
        .parse()
        .ok()?;
    Some(value.clamp(MIN_OPACITY, 1.0))
}

fn read_env_file(variable: &str) -> Option<String> {
    let path = std::env::var(variable).ok()?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => Some(contents),
        Err(error) => {
            log!("could not read {path}: {error}");
            None
        }
    }
}
