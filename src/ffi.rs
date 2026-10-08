use std::ffi::{c_char, c_void, CStr};
use std::mem::{size_of, transmute_copy};
use std::ptr;

pub type GType = usize;
pub type GBoolean = i32;
pub type GQuark = u32;

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
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
