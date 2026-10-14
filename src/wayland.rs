// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::log;

#[repr(C)]
pub struct Message {
    name: *const c_char,
    signature: *const c_char,
    types: *const *const Interface,
}

#[repr(C)]
pub struct Interface {
    name: *const c_char,
    version: c_int,
    method_count: c_int,
    methods: *const Message,
    event_count: c_int,
    events: *const Message,
}

#[link(name = "wayland-client")]
extern "C" {
    static wl_region_interface: Interface;
    static wl_registry_interface: Interface;

    fn wl_proxy_marshal_flags(
        proxy: *mut c_void,
        opcode: u32,
        interface: *const Interface,
        version: u32,
        flags: u32,
        ...
    ) -> *mut c_void;
    fn wl_proxy_get_version(proxy: *mut c_void) -> u32;
    fn wl_proxy_add_listener(
        proxy: *mut c_void,
        listener: *const *const c_void,
        data: *mut c_void,
    ) -> c_int;
    fn wl_proxy_destroy(proxy: *mut c_void);
    fn wl_proxy_set_queue(proxy: *mut c_void, queue: *mut c_void);
    fn wl_display_create_queue(display: *mut c_void) -> *mut c_void;
    fn wl_display_roundtrip_queue(display: *mut c_void, queue: *mut c_void) -> c_int;
    fn wl_event_queue_destroy(queue: *mut c_void);
}

const DESTROY_FLAG: u32 = 1;
const DISPLAY_GET_REGISTRY: u32 = 1;
const REGISTRY_BIND: u32 = 0;
const COMPOSITOR_CREATE_REGION: u32 = 1;
const REGION_DESTROY: u32 = 0;
const REGION_ADD: u32 = 1;
const MANAGER_DESTROY: u32 = 0;
const MANAGER_GET_BACKGROUND_EFFECT: u32 = 1;
const EFFECT_DESTROY: u32 = 0;
const EFFECT_SET_BLUR_REGION: u32 = 1;
const CAPABILITY_BLUR: u32 = 1;

static DISPLAY_CLOSED: AtomicBool = AtomicBool::new(false);

pub fn note_display_closed() {
    DISPLAY_CLOSED.store(true, Ordering::Relaxed);
    log!("the display is gone, no further wayland requests");
}

fn display_closed() -> bool {
    DISPLAY_CLOSED.load(Ordering::Relaxed)
}

struct Shared<T>(T);

unsafe impl<T> Sync for Shared<T> {}

static NO_TYPES: Shared<[*const Interface; 2]> = Shared([ptr::null(), ptr::null()]);

static EFFECT_METHODS: Shared<[Message; 2]> = Shared([
    Message {
        name: c"destroy".as_ptr(),
        signature: c"".as_ptr(),
        types: NO_TYPES.0.as_ptr(),
    },
    Message {
        name: c"set_blur_region".as_ptr(),
        signature: c"?o".as_ptr(),
        types: NO_TYPES.0.as_ptr(),
    },
]);

static EFFECT: Shared<Interface> = Shared(Interface {
    name: c"ext_background_effect_surface_v1".as_ptr(),
    version: 1,
    method_count: 2,
    methods: EFFECT_METHODS.0.as_ptr(),
    event_count: 0,
    events: ptr::null(),
});

static MANAGER_METHODS: Shared<[Message; 2]> = Shared([
    Message {
        name: c"destroy".as_ptr(),
        signature: c"".as_ptr(),
        types: NO_TYPES.0.as_ptr(),
    },
    Message {
        name: c"get_background_effect".as_ptr(),
        signature: c"no".as_ptr(),
        types: NO_TYPES.0.as_ptr(),
    },
]);

static MANAGER_EVENTS: Shared<[Message; 1]> = Shared([Message {
    name: c"capabilities".as_ptr(),
    signature: c"u".as_ptr(),
    types: NO_TYPES.0.as_ptr(),
}]);

static MANAGER: Shared<Interface> = Shared(Interface {
    name: c"ext_background_effect_manager_v1".as_ptr(),
    version: 1,
    method_count: 2,
    methods: MANAGER_METHODS.0.as_ptr(),
    event_count: 1,
    events: MANAGER_EVENTS.0.as_ptr(),
});

type RegistryGlobal = extern "C" fn(*mut c_void, *mut c_void, u32, *const c_char, u32);
type RegistryGlobalRemove = extern "C" fn(*mut c_void, *mut c_void, u32);
type ManagerCapabilities = extern "C" fn(*mut c_void, *mut c_void, u32);

static REGISTRY_LISTENER: Shared<[*const c_void; 2]> = Shared([
    on_global as RegistryGlobal as *const c_void,
    on_global_remove as RegistryGlobalRemove as *const c_void,
]);

static MANAGER_LISTENER: Shared<[*const c_void; 1]> =
    Shared([on_capabilities as ManagerCapabilities as *const c_void]);

struct Discovery {
    proxy: *mut c_void,
    capabilities: *mut u32,
}

extern "C" fn on_global(
    data: *mut c_void,
    registry: *mut c_void,
    name: u32,
    interface: *const c_char,
    version: u32,
) {
    unsafe {
        let discovery = data.cast::<Discovery>();
        if discovery.is_null() || interface.is_null() {
            return;
        }
        if !(*discovery).proxy.is_null() {
            return;
        }
        let wanted = &MANAGER.0;
        if CStr::from_ptr(interface) != CStr::from_ptr(wanted.name) {
            return;
        }
        let version = version.min(u32::try_from(wanted.version).unwrap_or(1));
        let manager = wl_proxy_marshal_flags(
            registry,
            REGISTRY_BIND,
            wanted,
            version,
            0,
            name,
            wanted.name,
            version,
            ptr::null_mut::<c_void>(),
        );
        if manager.is_null() {
            return;
        }
        wl_proxy_add_listener(
            manager,
            MANAGER_LISTENER.0.as_ptr(),
            (*discovery).capabilities.cast(),
        );
        (*discovery).proxy = manager;
        log!(
            "bound {} v{version}",
            CStr::from_ptr(interface).to_string_lossy()
        );
    }
}

extern "C" fn on_global_remove(_data: *mut c_void, _registry: *mut c_void, _name: u32) {}

extern "C" fn on_capabilities(data: *mut c_void, _manager: *mut c_void, flags: u32) {
    unsafe {
        if let Some(capabilities) = data.cast::<u32>().as_mut() {
            *capabilities = flags;
        }
    }
}

pub struct BlurManager {
    proxy: NonNull<c_void>,
    capabilities: Box<u32>,
}

impl BlurManager {
    pub unsafe fn bind(display: *mut c_void) -> Option<Self> {
        let queue = wl_display_create_queue(display);
        if queue.is_null() {
            return None;
        }
        let registry = wl_proxy_marshal_flags(
            display,
            DISPLAY_GET_REGISTRY,
            &raw const wl_registry_interface,
            wl_proxy_get_version(display),
            0,
            ptr::null_mut::<c_void>(),
        );
        if registry.is_null() {
            wl_event_queue_destroy(queue);
            return None;
        }

        let mut capabilities = Box::new(0u32);
        let mut discovery = Discovery {
            proxy: ptr::null_mut(),
            capabilities: &raw mut *capabilities,
        };
        wl_proxy_set_queue(registry, queue);
        wl_proxy_add_listener(
            registry,
            REGISTRY_LISTENER.0.as_ptr(),
            (&raw mut discovery).cast(),
        );
        wl_display_roundtrip_queue(display, queue);
        wl_display_roundtrip_queue(display, queue);
        wl_proxy_destroy(registry);

        let Discovery { proxy, .. } = discovery;
        let Some(proxy) = NonNull::new(proxy) else {
            log!("compositor does not advertise ext_background_effect_manager_v1");
            wl_event_queue_destroy(queue);
            return None;
        };
        wl_proxy_set_queue(proxy.as_ptr(), ptr::null_mut());
        wl_event_queue_destroy(queue);

        log!("manager bound, caps=0x{:x}", *capabilities);
        Some(Self {
            proxy,
            capabilities,
        })
    }

    pub fn capabilities(&self) -> u32 {
        *self.capabilities
    }

    pub fn supports_blur(&self) -> bool {
        self.capabilities() & CAPABILITY_BLUR != 0
    }

    pub unsafe fn effect_for(&self, surface: *mut c_void) -> Option<BlurEffect> {
        if display_closed() {
            return None;
        }
        let proxy = wl_proxy_marshal_flags(
            self.proxy.as_ptr(),
            MANAGER_GET_BACKGROUND_EFFECT,
            &EFFECT.0,
            wl_proxy_get_version(self.proxy.as_ptr()),
            0,
            ptr::null_mut::<c_void>(),
            surface,
        );
        NonNull::new(proxy).map(|proxy| BlurEffect { proxy })
    }
}

impl Drop for BlurManager {
    fn drop(&mut self) {
        if display_closed() {
            return;
        }
        unsafe {
            wl_proxy_marshal_flags(
                self.proxy.as_ptr(),
                MANAGER_DESTROY,
                ptr::null(),
                wl_proxy_get_version(self.proxy.as_ptr()),
                DESTROY_FLAG,
            );
        }
    }
}

pub struct BlurEffect {
    proxy: NonNull<c_void>,
}

impl BlurEffect {
    pub unsafe fn set_region(&self, compositor: *mut c_void, extent: i32) {
        if display_closed() || compositor.is_null() {
            return;
        }
        let region = wl_proxy_marshal_flags(
            compositor,
            COMPOSITOR_CREATE_REGION,
            &raw const wl_region_interface,
            wl_proxy_get_version(compositor),
            0,
            ptr::null_mut::<c_void>(),
        );
        if region.is_null() {
            return;
        }
        wl_proxy_marshal_flags(
            region,
            REGION_ADD,
            ptr::null(),
            wl_proxy_get_version(region),
            0,
            0,
            0,
            extent,
            extent,
        );
        wl_proxy_marshal_flags(
            self.proxy.as_ptr(),
            EFFECT_SET_BLUR_REGION,
            ptr::null(),
            wl_proxy_get_version(self.proxy.as_ptr()),
            0,
            region,
        );
        wl_proxy_marshal_flags(
            region,
            REGION_DESTROY,
            ptr::null(),
            wl_proxy_get_version(region),
            DESTROY_FLAG,
        );
    }
}

impl Drop for BlurEffect {
    fn drop(&mut self) {
        if display_closed() {
            return;
        }
        unsafe {
            wl_proxy_marshal_flags(
                self.proxy.as_ptr(),
                EFFECT_DESTROY,
                ptr::null(),
                wl_proxy_get_version(self.proxy.as_ptr()),
                DESTROY_FLAG,
            );
        }
    }
}
