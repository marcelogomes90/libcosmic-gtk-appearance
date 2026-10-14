// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

pub mod cosmic;
mod ffi;
mod gtk;
pub mod style;
mod toolkit;
mod watch;
mod wayland;

use std::cell::RefCell;
use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::OnceLock;
use std::thread::ThreadId;

use cosmic::{Appearance, Decorations, Glass};
use ffi::{GBoolean, Library};
use gtk::{Gtk, ObjectRefs};
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
const PROCESS_OWNER_KEY: &CStr = c"libcosmic-gtk-appearance-owner";
const DISPLAY_CLOSED_SIGNAL: &CStr = c"closed";
const SOURCE_REMOVE: GBoolean = 0;
const KEEP_EMISSION_HOOK: GBoolean = 1;
const BLUR_EXTENT: i32 = 32767;
const SELF_PAINTING_ENGINES: [&CStr; 1] = [c"libxul.so"];

static CLAIM_TOKEN: u8 = 1;
static OBJECT_REFS: OnceLock<ObjectRefs> = OnceLock::new();

struct Session {
    library: Library,
    gtk: Gtk,
    toolkit: Toolkit,
    wayland: Option<Wayland>,
    searched_for_wayland: bool,
    compositor: *mut c_void,
    blur: Option<BlurManager>,
    blur_display: *mut c_void,
    searched_for_blur: bool,
    decorations: Option<Decorations>,
    glass: Option<Glass>,
    styling: bool,
    provider: *mut c_void,
    provider_display: *mut c_void,
    css: Option<CString>,
    frosted: bool,
    stale: bool,
    watching: bool,
    watched: Vec<std::path::PathBuf>,
    owner: ThreadId,
}

struct SessionCell(RefCell<Option<Session>>);

unsafe impl Sync for SessionCell {}

static SESSION: SessionCell = SessionCell(RefCell::new(None));

fn with_session<T>(act: impl FnOnce(&Session) -> T) -> Option<T> {
    let Ok(borrowed) = SESSION.0.try_borrow() else {
        log!("skipping a re-entrant call into the session");
        return None;
    };
    let session = borrowed.as_ref()?;
    if session.owner != std::thread::current().id() {
        log!("ignoring a call from a thread that did not start the session");
        return None;
    }
    Some(act(session))
}

fn with_session_mut<T>(act: impl FnOnce(&mut Session) -> T) -> Option<T> {
    let Ok(mut borrowed) = SESSION.0.try_borrow_mut() else {
        log!("skipping a re-entrant call into the session");
        return None;
    };
    let session = borrowed.as_mut()?;
    if session.owner != std::thread::current().id() {
        log!("ignoring a call from a thread that did not start the session");
        return None;
    }
    Some(act(session))
}

struct WindowBlur {
    effect: Option<BlurEffect>,
    wl_surface: *mut c_void,
}

extern "C" fn release_window_blur(data: *mut c_void) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !data.is_null() {
            unsafe { drop(Box::from_raw(data.cast::<WindowBlur>())) }
        }
    }));
}

extern "C" fn on_display_closed(_display: *mut c_void, _is_error: GBoolean, _data: *mut c_void) {
    let _ = catch_unwind(wayland::note_display_closed);
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

unsafe fn paints_its_own_background() -> bool {
    let Some(engine) = SELF_PAINTING_ENGINES
        .into_iter()
        .find(|soname| Library::resident(soname).is_some())
    else {
        return false;
    };
    log!(
        "{} paints its own window background, leaving this process alone",
        engine.to_string_lossy()
    );
    true
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
    if disabled || !on_wayland || paints_its_own_background() {
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

unsafe fn claim_this_process(gtk: &Gtk) -> bool {
    let quark = (gtk.quark_from_static_string)(PROCESS_OWNER_KEY.as_ptr());
    if !(gtk.type_get_qdata)(gtk.window_type, quark).is_null() {
        return false;
    }
    (gtk.type_set_qdata)(
        gtk.window_type,
        quark,
        (&raw const CLAIM_TOKEN).cast_mut().cast(),
    );
    true
}

unsafe fn start_session() -> Option<()> {
    if SESSION.0.try_borrow().is_ok_and(|held| held.is_some()) {
        return Some(());
    }

    if paints_its_own_background() {
        return None;
    }
    let Some((library, soname)) = resident_toolkit() else {
        log!("no GTK in this process, nothing to do");
        return None;
    };
    let gtk = Gtk::load(&library)?;
    let toolkit = Toolkit::load(&library, soname)?;
    let _ = OBJECT_REFS.set(ObjectRefs::load(&library)?);
    if !claim_this_process(&gtk) {
        log!("another copy of this library already drives this process, standing down");
        return None;
    }
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

    let session = Session {
        library,
        gtk,
        toolkit,
        wayland: None,
        searched_for_wayland: false,
        compositor: ptr::null_mut(),
        blur: None,
        blur_display: ptr::null_mut(),
        searched_for_blur: false,
        decorations: appearance.as_ref().map(|found| found.decorations),
        glass,
        styling: std::env::var_os("COSMIC_GTK_APPEARANCE_NO_CSS").is_none(),
        provider: ptr::null_mut(),
        provider_display: ptr::null_mut(),
        css: None,
        frosted: false,
        stale: true,
        watching: false,
        watched: Vec::new(),
        owner: std::thread::current().id(),
    };
    *SESSION.0.try_borrow_mut().ok()? = Some(session);

    with_session(|session| {
        (session.gtk.signal_add_emission_hook)(
            session.gtk.map_signal,
            0,
            on_window_mapped,
            ptr::null_mut(),
            ptr::null(),
        );
        each_toplevel(session, |window| defer_apply(session, window));
    })
}

extern "C" fn on_window_mapped(
    _hint: *mut c_void,
    parameter_count: c_uint,
    parameters: *const c_void,
    _data: *mut c_void,
) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if parameter_count < 1 {
            return;
        }
        with_session(|session| unsafe {
            let instance = (session.gtk.value_get_object)(parameters);
            if instance.is_null() || !session.gtk.is_window(instance) {
                return;
            }
            defer_apply(session, instance);
        });
    }));
    KEEP_EMISSION_HOOK
}

unsafe fn each_toplevel(session: &Session, visit: impl FnMut(*mut c_void)) {
    let toplevels = (session.gtk.window_list_toplevels)();
    session.gtk.each_in_list(toplevels, visit);
}

unsafe fn defer_apply(session: &Session, window: *mut c_void) {
    if window.is_null() {
        return;
    }
    let Some(refs) = OBJECT_REFS.get() else {
        return;
    };
    (refs.object_ref)(window);
    (session.gtk.idle_add)(apply_to_window, window);
}

extern "C" fn apply_to_window(window: *mut c_void) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        with_session_mut(|session| unsafe { session.apply(window) });
    }));
    if let Some(refs) = OBJECT_REFS.get() {
        let _ = catch_unwind(AssertUnwindSafe(|| (refs.object_unref)(window)));
    }
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
        if self.styling && frosted {
            self.toolkit.mark_sidebar_titlebars(&self.gtk, window);
        }
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
        if self.blur_display != display {
            log!("surface belongs to another display, leaving its blur alone");
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
                self.blur_display = display;
                self.blur = BlurManager::bind(wl_display);
                self.watch_display(display);
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

    unsafe fn watch_display(&self, display: *mut c_void) {
        (self.gtk.signal_connect_data)(
            display,
            DISPLAY_CLOSED_SIGNAL.as_ptr(),
            on_display_closed as *const c_void,
            ptr::null_mut(),
            ptr::null(),
            0,
        );
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
        let display = self.toolkit.display_of(surface);
        let same_display = self.provider.is_null() || self.provider_display == display;
        if !self.stale && same_display && self.frosted == frosted {
            return;
        }
        let Some(css) = self.compose_stylesheet(frosted) else {
            log!("could not compose a stylesheet, leaving the last one in place");
            return;
        };
        if same_display && self.css.as_deref() == Some(css.as_c_str()) {
            self.frosted = frosted;
            self.stale = false;
            return;
        }
        if self.provider.is_null() || !same_display {
            let provider = self.toolkit.add_stylesheet(&self.gtk, surface, &css);
            if provider.is_null() {
                return;
            }
            self.provider = provider;
            self.provider_display = display;
        } else {
            self.toolkit.load_stylesheet(self.provider, &css);
        }
        self.css = Some(css);
        self.frosted = frosted;
        self.stale = false;
        log!(
            "stylesheet applied ({}, frosted glass {})",
            self.toolkit.name(),
            if frosted { "on" } else { "off" }
        );
    }

    fn compose_stylesheet(&self, frosted: bool) -> Option<CString> {
        let glass = if frosted { self.glass } else { None };
        let mut css = cosmic::palette(self.toolkit.config_dir())
            .unwrap_or_else(|| self.toolkit.fallback_palette().to_string());
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
        match CString::new(css) {
            Ok(text) => Some(text),
            Err(error) => {
                log!(
                    "the stylesheet holds a nul byte at {}",
                    error.nul_position()
                );
                None
            }
        }
    }

    unsafe fn start_watching(&mut self) {
        let directories = cosmic::watched_directories(self.toolkit.config_dir());
        if self.watching && directories == self.watched {
            return;
        }
        let Some(add_source) = self.gtk.unix_fd_add_full else {
            return;
        };
        let Some(timeout_add) = self.gtk.timeout_add else {
            return;
        };
        let borrowed: Vec<&std::path::Path> = directories.iter().map(AsRef::as_ref).collect();
        if watch::directories(&borrowed, add_source, timeout_add, on_appearance_changed) {
            self.watching = true;
            self.watched = directories;
        }
    }
}

fn on_appearance_changed() {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        with_session_mut(|session| {
            let appearance = Appearance::from_config();
            session.decorations = appearance.as_ref().map(|found| found.decorations);
            session.glass = resolve_glass(appearance.as_ref());
            session.stale = true;
            log!(
                "appearance changed, frosted glass {}",
                if session.glass.is_some() { "on" } else { "off" }
            );
        });
        with_session(|session| unsafe {
            each_toplevel(session, |window| defer_apply(session, window));
        });
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
