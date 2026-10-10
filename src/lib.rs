// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

mod cosmic;
mod ffi;
mod gtk;
mod style;
mod toolkit;
mod watch;
mod wayland;

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::OnceLock;

use cosmic::{Appearance, Decorations, Glass};
use ffi::{GBoolean, Library};
use gtk::Gtk;
use toolkit::{Toolkit, Wayland};
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
const GDK_SKIP_PROTOCOLS: &str = "GDK_WAYLAND_DISABLE";
const BLUR_MANAGER: &str = "ext_background_effect_manager_v1";
const SKIP_SEPARATORS: [char; 5] = [':', ';', ',', ' ', '\t'];
const WINDOW_DATA_KEY: &CStr = c"libcosmic-gtk-appearance";
const SOURCE_REMOVE: GBoolean = 0;
const KEEP_EMISSION_HOOK: GBoolean = 1;
const BLUR_EXTENT: i32 = 32767;

struct Session {
    library: Library,
    gtk: Gtk,
    toolkit: Toolkit,
    wayland: Option<Wayland>,
    searched_for_wayland: bool,
    compositor: *mut c_void,
    blur: Option<BlurManager>,
    searched_for_blur: bool,
    decorations: Option<Decorations>,
    glass: Option<Glass>,
    styling: bool,
    provider: *mut c_void,
    css: Option<CString>,
    frosted: bool,
    stale: bool,
    watching: bool,
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
        unsafe { drop(Box::from_raw(data.cast::<WindowBlur>())) }
    }
}

#[used]
#[link_section = ".init_array"]
static INITIALIZER: extern "C" fn() = on_library_loaded;

extern "C" fn on_library_loaded() {
    let _ = catch_unwind(|| unsafe { schedule_start() });
}

#[no_mangle]
pub extern "C" fn g_io_module_load(_module: *mut c_void) {}

#[no_mangle]
pub extern "C" fn g_io_module_unload(_module: *mut c_void) {}

#[no_mangle]
pub extern "C" fn g_io_module_query() -> *mut *mut c_char {
    ptr::null_mut()
}

unsafe fn resident_toolkit() -> Option<(Library, &'static CStr)> {
    toolkit::SONAMES
        .into_iter()
        .find_map(|soname| Library::resident(soname).map(|library| (library, soname)))
}

static CLAIMED_BLUR: OnceLock<bool> = OnceLock::new();

pub(crate) fn claimed_blur() -> bool {
    CLAIMED_BLUR.get().copied().unwrap_or(false)
}

fn blur_manager_is_skipped(listing: &str) -> bool {
    listing
        .split(SKIP_SEPARATORS)
        .any(|name| name == BLUR_MANAGER)
}

fn listing_with_blur_manager(listing: &str) -> String {
    if listing.is_empty() {
        BLUR_MANAGER.to_string()
    } else {
        format!("{listing},{BLUR_MANAGER}")
    }
}

unsafe fn claim_blur() {
    CLAIMED_BLUR.get_or_init(|| {
        let listing = std::env::var(GDK_SKIP_PROTOCOLS).unwrap_or_default();
        if blur_manager_is_skipped(&listing) {
            log!("GDK was told to skip {BLUR_MANAGER}, the blur is ours to drive");
            return true;
        }
        if resident_toolkit().is_some() {
            log!("GTK is already loaded, the blur stays with it");
            return false;
        }
        std::env::set_var(GDK_SKIP_PROTOCOLS, listing_with_blur_manager(&listing));
        log!("asked GDK to skip {BLUR_MANAGER}");
        true
    });
}

unsafe fn schedule_start() -> Option<()> {
    let disabled = std::env::var_os("COSMIC_GTK_APPEARANCE_DISABLE").is_some();
    let on_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if disabled || !on_wayland {
        return None;
    }
    claim_blur();
    let glib = Library::resident(c"libglib-2.0.so.0").unwrap_or_else(Library::global);
    let idle_add: extern "C" fn(gtk::SourceFunc, *mut c_void) -> c_uint =
        glib.symbol(c"g_idle_add")?;
    if !ffi::pin_in_memory(on_library_loaded as *const c_void) {
        log!("could not pin the library in memory, giving up");
        return None;
    }
    idle_add(start, ptr::null_mut());
    Some(())
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

    let Some((library, soname)) = resident_toolkit() else {
        log!("no GTK in this process, nothing to do");
        return None;
    };
    let gtk = Gtk::load(&library)?;
    let toolkit = Toolkit::load(&library, soname)?;
    let appearance = Appearance::from_config();
    let glass = resolve_glass(appearance.as_ref());
    log!(
        "toolkit {}, COSMIC theme {}, frosted glass {}, palette {}",
        toolkit.name(),
        if appearance.is_some() {
            "loaded"
        } else {
            "not found, falling back to GTK colours"
        },
        if glass.is_some() { "on" } else { "off" },
        if cosmic::palette(toolkit.config_dir()).is_some() {
            "followed"
        } else {
            "left to GTK"
        }
    );

    *SESSION.0.get() = Some(Session {
        library,
        gtk,
        toolkit,
        wayland: None,
        searched_for_wayland: false,
        compositor: ptr::null_mut(),
        blur: None,
        searched_for_blur: false,
        decorations: appearance.as_ref().map(|found| found.decorations),
        glass,
        styling: std::env::var_os("COSMIC_GTK_APPEARANCE_NO_CSS").is_none(),
        provider: ptr::null_mut(),
        css: None,
        frosted: false,
        stale: true,
        watching: false,
    });

    let session = session_ref()?;
    (session.gtk.signal_add_emission_hook)(
        session.gtk.map_signal,
        0,
        on_window_mapped,
        ptr::null_mut(),
        ptr::null(),
    );
    each_toplevel(session, |window| defer_apply(session, window));
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
        defer_apply(session, instance);
    }));
    KEEP_EMISSION_HOOK
}

unsafe fn each_toplevel(session: &Session, mut visit: impl FnMut(*mut c_void)) {
    let toplevels = (session.gtk.window_list_toplevels)();
    let mut node = toplevels;
    while !node.is_null() {
        visit((*node).data);
        node = (*node).next;
    }
    if !toplevels.is_null() {
        (session.gtk.list_free)(toplevels);
    }
}

unsafe fn defer_apply(session: &Session, window: *mut c_void) {
    if window.is_null() {
        return;
    }
    (session.gtk.object_ref)(window);
    (session.gtk.idle_add)(apply_to_window, window);
}

extern "C" fn apply_to_window(window: *mut c_void) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        if let Some(session) = session_mut() {
            session.apply(window);
            (session.gtk.object_unref)(window);
        }
    }));
    SOURCE_REMOVE
}

impl Session {
    unsafe fn apply(&mut self, window: *mut c_void) -> Option<()> {
        if !self.gtk.is_mapped(window) {
            return None;
        }
        if !self.toolkit.is_decorated_toplevel(window) {
            log!("skipping popup window {window:p}");
            return None;
        }
        let surface = self.toolkit.surface_of(window);
        if surface.is_null() {
            return None;
        }

        let frosted = self.attach_blur(window, surface);
        self.refresh_stylesheet(surface, frosted);
        self.start_watching();
        Some(())
    }

    unsafe fn attach_blur(&mut self, window: *mut c_void, surface: *mut c_void) -> bool {
        if !self.searched_for_wayland {
            self.searched_for_wayland = true;
            self.wayland = Wayland::load(&self.library, &self.toolkit);
            if self.wayland.is_none() {
                log!("this GTK build has no wayland backend");
            }
        }
        let Some(wayland) = self.wayland.as_ref() else {
            return false;
        };
        let wl_surface = wayland.wl_surface_of(&self.gtk, surface);
        if wl_surface.is_null() {
            log!("surface does not come from the wayland backend");
            return false;
        }
        if self.glass.is_none() {
            self.detach_blur(surface);
            return false;
        }
        let display = self.toolkit.display_of(surface);
        if display.is_null() {
            return false;
        }
        if !self.connect_blur(display) {
            return false;
        }
        if self.toolkit.gtk_owns_blur() {
            return true;
        }

        let record = self.window_blur(surface);
        if (*record).wl_surface == wl_surface {
            return true;
        }
        (*record).effect = None;
        (*record).wl_surface = ptr::null_mut();

        let Some(effect) = self.blur.as_ref().and_then(|m| m.effect_for(wl_surface)) else {
            log!("get_background_effect failed");
            return false;
        };
        effect.set_region(self.compositor, BLUR_EXTENT);
        (*record).effect = Some(effect);
        (*record).wl_surface = wl_surface;
        (self.gtk.widget_queue_draw)(window);
        log!("blur attached to window {window:p}");
        true
    }

    unsafe fn connect_blur(&mut self, display: *mut c_void) -> bool {
        if !self.searched_for_blur {
            self.searched_for_blur = true;
            let Some(wayland) = self.wayland.as_ref() else {
                return false;
            };
            let wl_display = (wayland.display_get_wl_display)(display);
            let compositor = (wayland.display_get_wl_compositor)(display);
            if wl_display.is_null() || compositor.is_null() {
                log!("no wl_display or wl_compositor");
            } else {
                self.compositor = compositor;
                self.blur = BlurManager::bind(wl_display);
            }
        }
        match self.blur.as_ref() {
            None => false,
            Some(manager) if !manager.supports_blur() => {
                log!(
                    "compositor lacks the blur capability (caps=0x{:x})",
                    manager.capabilities()
                );
                false
            }
            Some(_) => true,
        }
    }

    unsafe fn detach_blur(&self, surface: *mut c_void) {
        let stored = self.stored_blur(surface);
        if let Some(record) = stored.as_mut() {
            if record.effect.take().is_some() {
                record.wl_surface = ptr::null_mut();
                log!("blur released, frosted glass is off");
            }
        }
    }

    unsafe fn stored_blur(&self, surface: *mut c_void) -> *mut WindowBlur {
        (self.gtk.object_get_data)(surface, WINDOW_DATA_KEY.as_ptr()).cast::<WindowBlur>()
    }

    unsafe fn window_blur(&self, surface: *mut c_void) -> *mut WindowBlur {
        let stored = self.stored_blur(surface);
        if !stored.is_null() {
            return stored;
        }
        let fresh = Box::into_raw(Box::new(WindowBlur {
            effect: None,
            wl_surface: ptr::null_mut(),
        }));
        (self.gtk.object_set_data_full)(
            surface,
            WINDOW_DATA_KEY.as_ptr(),
            fresh.cast(),
            release_window_blur as *const c_void,
        );
        fresh
    }

    unsafe fn refresh_stylesheet(&mut self, surface: *mut c_void, frosted: bool) {
        if !self.styling {
            return;
        }
        if !self.stale && !self.provider.is_null() && self.frosted == frosted {
            return;
        }
        self.frosted = frosted;
        self.stale = false;
        let Some(css) = self.compose_stylesheet(frosted) else {
            return;
        };
        if self.css.as_deref() == Some(css.as_c_str()) {
            return;
        }
        if self.provider.is_null() {
            self.provider = self.toolkit.add_stylesheet(&self.gtk, surface, &css);
            if self.provider.is_null() {
                return;
            }
        } else {
            self.toolkit.load_stylesheet(self.provider, &css);
        }
        self.css = Some(css);
        log!(
            "stylesheet applied ({}, frosted glass {})",
            self.toolkit.name(),
            if frosted { "on" } else { "off" }
        );
    }

    fn compose_stylesheet(&self, frosted: bool) -> Option<CString> {
        let glass = if frosted { self.glass } else { None };
        let mut css = cosmic::palette(self.toolkit.config_dir()).unwrap_or_default();
        css.push_str(
            &read_env_file("COSMIC_GTK_APPEARANCE_CSS").unwrap_or_else(|| {
                self.toolkit
                    .stylesheet(glass.as_ref(), self.decorations.as_ref())
            }),
        );
        if let Some(extra) = read_env_file("COSMIC_GTK_APPEARANCE_CSS_EXTRA") {
            css.push('\n');
            css.push_str(&extra);
        }
        CString::new(css).ok()
    }

    unsafe fn start_watching(&mut self) {
        if self.watching {
            return;
        }
        let directories = cosmic::watched_directories(self.toolkit.config_dir());
        let borrowed: Vec<&std::path::Path> = directories.iter().map(AsRef::as_ref).collect();
        self.watching = watch::directories(
            &borrowed,
            self.gtk.unix_fd_add_full,
            self.gtk.timeout_add,
            on_appearance_changed,
        );
    }
}

fn on_appearance_changed() {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        {
            let Some(session) = session_mut() else {
                return;
            };
            let appearance = Appearance::from_config();
            session.decorations = appearance.as_ref().map(|found| found.decorations);
            session.glass = resolve_glass(appearance.as_ref());
            session.stale = true;
            log!(
                "appearance changed, frosted glass {}",
                if session.glass.is_some() { "on" } else { "off" }
            );
        }
        let Some(session) = session_ref() else {
            return;
        };
        each_toplevel(session, |window| defer_apply(session, window));
    }));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_that_names_the_manager_hands_us_the_blur() {
        for listing in [
            BLUR_MANAGER,
            "wp_viewporter:ext_background_effect_manager_v1",
            "a,ext_background_effect_manager_v1,b",
            "wp_viewporter ext_background_effect_manager_v1",
        ] {
            assert!(blur_manager_is_skipped(listing));
        }
    }

    #[test]
    fn a_name_that_only_looks_like_it_does_not() {
        for listing in ["", "wp_viewporter", "ext_background_effect_manager_v10"] {
            assert!(!blur_manager_is_skipped(listing));
        }
    }

    #[test]
    fn asking_for_the_manager_keeps_what_was_already_listed() {
        assert_eq!(listing_with_blur_manager(""), BLUR_MANAGER);
        assert!(blur_manager_is_skipped(&listing_with_blur_manager(
            "wp_viewporter"
        )));
        assert!(listing_with_blur_manager("wp_viewporter").starts_with("wp_viewporter"));
    }
}
