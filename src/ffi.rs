// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::ffi::{c_char, c_void, CStr};
use std::mem::{size_of, transmute_copy};
use std::ptr;

pub type GType = usize;
pub type GBoolean = i32;
pub type GQuark = u32;

#[repr(C)]
struct SharedObjectInfo {
    path: *const c_char,
    base: *mut c_void,
    symbol_name: *const c_char,
    symbol_address: *mut c_void,
}

const RTLD_NOW: i32 = 0x2;
const RTLD_NODELETE: i32 = 0x1000;

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlopen(path: *const c_char, flags: i32) -> *mut c_void;
    fn dladdr(address: *const c_void, info: *mut SharedObjectInfo) -> i32;
}

pub unsafe fn pin_in_memory(anchor: *const c_void) -> bool {
    let mut info = SharedObjectInfo {
        path: ptr::null(),
        base: ptr::null_mut(),
        symbol_name: ptr::null(),
        symbol_address: ptr::null_mut(),
    };
    if dladdr(anchor, &raw mut info) == 0 || info.path.is_null() {
        return false;
    }
    !dlopen(info.path, RTLD_NOW | RTLD_NODELETE).is_null()
}

pub unsafe fn symbol(name: &CStr) -> Option<*mut c_void> {
    let address = dlsym(ptr::null_mut(), name.as_ptr());
    (!address.is_null()).then_some(address)
}

pub unsafe fn has_symbol(name: &CStr) -> bool {
    symbol(name).is_some()
}

pub unsafe fn function<T: Copy>(name: &CStr) -> Option<T> {
    debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    symbol(name).map(|address| transmute_copy(&address))
}

macro_rules! resolve {
    ($name:expr) => {
        match $crate::ffi::function($name) {
            Some(address) => address,
            None => {
                $crate::log!("missing symbol: {}", $name.to_string_lossy());
                return None;
            }
        }
    };
}

pub(crate) use resolve;
