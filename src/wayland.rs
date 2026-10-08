// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::OnceLock;

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

unsafe impl Sync for Message {}
unsafe impl Sync for Interface {}

#[link(name = "wayland-client")]
extern "C" {
    static wl_surface_interface: Interface;
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
    fn wl_proxy_set_queue(proxy: *mut c_void, queue: *mut c_void);
    fn wl_display_create_queue(display: *mut c_void) -> *mut c_void;
    fn wl_display_roundtrip_queue(display: *mut c_void, queue: *mut c_void) -> c_int;
}

const DESTROY_FLAG: u32 = 1;
const DISPLAY_GET_REGISTRY: u32 = 1;
const REGISTRY_BIND: u32 = 0;
const COMPOSITOR_CREATE_REGION: u32 = 1;
const REGION_DESTROY: u32 = 0;
const REGION_ADD: u32 = 1;
const MANAGER_GET_BACKGROUND_EFFECT: u32 = 1;
const EFFECT_DESTROY: u32 = 0;
const EFFECT_SET_BLUR_REGION: u32 = 1;
const CAPABILITY_BLUR: u32 = 1;

type RegistryGlobal = extern "C" fn(*mut c_void, *mut c_void, u32, *const c_char, u32);
type RegistryGlobalRemove = extern "C" fn(*mut c_void, *mut c_void, u32);
type ManagerCapabilities = extern "C" fn(*mut c_void, *mut c_void, u32);

#[repr(C)]
struct BindTarget {
    interface: *const Interface,
    listener: *const *const c_void,
}

struct Protocol {
    effect: &'static Interface,
    registry_listener: &'static [*const c_void; 2],
    bind_target: &'static BindTarget,
}

unsafe impl Sync for Protocol {}
unsafe impl Send for Protocol {}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

unsafe fn describe_protocol() -> Protocol {
    let no_types: &'static [*const Interface; 1] = leak([ptr::null()]);
    let region_type: &'static [*const Interface; 1] = leak([ptr::addr_of!(wl_region_interface)]);

    let effect_methods: &'static [Message; 2] = leak([
        Message {
            name: c"destroy".as_ptr(),
            signature: c"".as_ptr(),
            types: no_types.as_ptr(),
        },
        Message {
            name: c"set_blur_region".as_ptr(),
            signature: c"?o".as_ptr(),
            types: region_type.as_ptr(),
        },
    ]);
    let effect: &'static Interface = leak(Interface {
        name: c"ext_background_effect_surface_v1".as_ptr(),
        version: 1,
        method_count: 2,
        methods: effect_methods.as_ptr(),
        event_count: 0,
        events: ptr::null(),
    });

    let get_effect_types: &'static [*const Interface; 2] = leak([
        effect as *const Interface,
        ptr::addr_of!(wl_surface_interface),
    ]);
    let manager_methods: &'static [Message; 2] = leak([
        Message {
            name: c"destroy".as_ptr(),
            signature: c"".as_ptr(),
            types: no_types.as_ptr(),
        },
        Message {
            name: c"get_background_effect".as_ptr(),
            signature: c"no".as_ptr(),
            types: get_effect_types.as_ptr(),
        },
    ]);
    let manager_events: &'static [Message; 1] = leak([Message {
        name: c"capabilities".as_ptr(),
        signature: c"u".as_ptr(),
        types: no_types.as_ptr(),
    }]);
    let manager: &'static Interface = leak(Interface {
        name: c"ext_background_effect_manager_v1".as_ptr(),
        version: 1,
        method_count: 2,
        methods: manager_methods.as_ptr(),
        event_count: 1,
        events: manager_events.as_ptr(),
    });

    let manager_listener: &'static [*const c_void; 1] =
        leak([on_capabilities as ManagerCapabilities as *const c_void]);

    Protocol {
        effect,
        registry_listener: leak([
            on_global as RegistryGlobal as *const c_void,
            on_global_remove as RegistryGlobalRemove as *const c_void,
        ]),
        bind_target: leak(BindTarget {
            interface: manager,
            listener: manager_listener.as_ptr(),
        }),
    }
}

static PROTOCOL: OnceLock<Protocol> = OnceLock::new();

fn protocol() -> &'static Protocol {
    PROTOCOL.get_or_init(|| unsafe { describe_protocol() })
}

struct Discovery(UnsafeCell<(*mut c_void, u32)>);

unsafe impl Sync for Discovery {}

static DISCOVERY: Discovery = Discovery(UnsafeCell::new((ptr::null_mut(), 0)));

extern "C" fn on_global(
    data: *mut c_void,
    registry: *mut c_void,
    name: u32,
    interface: *const c_char,
    version: u32,
) {
    unsafe {
        let target = data as *const BindTarget;
        if target.is_null() || interface.is_null() {
            return;
        }
        let wanted = (*target).interface;
        if wanted.is_null() || CStr::from_ptr(interface) != CStr::from_ptr((*wanted).name) {
            return;
        }
        let version = version.min((*wanted).version as u32);
        let manager = wl_proxy_marshal_flags(
            registry,
            REGISTRY_BIND,
            wanted,
            version,
            0,
            name,
            (*wanted).name,
            version,
            ptr::null_mut::<c_void>(),
        );
        if manager.is_null() {
            return;
        }
        wl_proxy_add_listener(manager, (*target).listener, ptr::null_mut());
        (*DISCOVERY.0.get()).0 = manager;
        log!(
            "bound {} v{version}",
            CStr::from_ptr(interface).to_string_lossy()
        );
    }
}

extern "C" fn on_global_remove(_data: *mut c_void, _registry: *mut c_void, _name: u32) {}

extern "C" fn on_capabilities(_data: *mut c_void, _manager: *mut c_void, flags: u32) {
    unsafe {
        (*DISCOVERY.0.get()).1 = flags;
    }
}

pub struct BlurManager {
    proxy: *mut c_void,
    capabilities: u32,
}

impl BlurManager {
    pub unsafe fn bind(display: *mut c_void) -> Option<Self> {
        let protocol = protocol();
        let queue = wl_display_create_queue(display);
        if queue.is_null() {
            return None;
        }
        let registry = wl_proxy_marshal_flags(
            display,
            DISPLAY_GET_REGISTRY,
            ptr::addr_of!(wl_registry_interface),
            wl_proxy_get_version(display),
            0,
            ptr::null_mut::<c_void>(),
        );
        if registry.is_null() {
            return None;
        }
        wl_proxy_set_queue(registry, queue);
        wl_proxy_add_listener(
            registry,
            protocol.registry_listener.as_ptr(),
            protocol.bind_target as *const BindTarget as *mut c_void,
        );

        wl_display_roundtrip_queue(display, queue);
        if (*DISCOVERY.0.get()).0.is_null() {
            log!("compositor does not advertise ext_background_effect_manager_v1");
            return None;
        }
        wl_display_roundtrip_queue(display, queue);

        let (proxy, capabilities) = *DISCOVERY.0.get();
        log!("manager bound, caps=0x{capabilities:x}");
        Some(Self {
            proxy,
            capabilities,
        })
    }

    pub fn supports_blur(&self) -> bool {
        self.capabilities & CAPABILITY_BLUR != 0
    }

    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }

    pub unsafe fn effect_for(&self, surface: *mut c_void) -> Option<BlurEffect> {
        let proxy = wl_proxy_marshal_flags(
            self.proxy,
            MANAGER_GET_BACKGROUND_EFFECT,
            protocol().effect,
            wl_proxy_get_version(self.proxy),
            0,
            ptr::null_mut::<c_void>(),
            surface,
        );
        (!proxy.is_null()).then_some(BlurEffect { proxy })
    }
}

pub struct BlurEffect {
    proxy: *mut c_void,
}

impl BlurEffect {
    pub unsafe fn set_region(&self, compositor: *mut c_void, extent: i32) {
        let region = wl_proxy_marshal_flags(
            compositor,
            COMPOSITOR_CREATE_REGION,
            ptr::addr_of!(wl_region_interface),
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
            self.proxy,
            EFFECT_SET_BLUR_REGION,
            ptr::null(),
            wl_proxy_get_version(self.proxy),
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

    pub unsafe fn destroy(self) {
        wl_proxy_marshal_flags(
            self.proxy,
            EFFECT_DESTROY,
            ptr::null(),
            wl_proxy_get_version(self.proxy),
            DESTROY_FLAG,
        );
    }
}
