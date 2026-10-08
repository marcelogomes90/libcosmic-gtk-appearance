// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::path::Path;
use std::ptr;

use crate::ffi;
use crate::log;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOSE_WRITE: u32 = 0x0000_0008;
const IN_MOVED_TO: u32 = 0x0000_0080;
const IN_CREATE: u32 = 0x0000_0100;
const G_IO_IN: c_uint = 1;
const G_PRIORITY_DEFAULT: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;

extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

type FdSourceFunc = extern "C" fn(c_int, c_uint, *mut c_void) -> c_int;
type AddFdSource =
    extern "C" fn(c_int, c_int, c_uint, FdSourceFunc, *mut c_void, *const c_void) -> c_uint;

struct Handler(UnsafeCell<Option<fn()>>);

unsafe impl Sync for Handler {}

static HANDLER: Handler = Handler(UnsafeCell::new(None));

extern "C" fn drain(fd: c_int, _condition: c_uint, _data: *mut c_void) -> c_int {
    let mut buffer = [0u8; 4096];
    unsafe {
        while read(fd, buffer.as_mut_ptr().cast(), buffer.len()) > 0 {}
        if let Some(handler) = *HANDLER.0.get() {
            handler();
        }
    }
    G_SOURCE_CONTINUE
}

pub unsafe fn directories(paths: &[&Path], handler: fn()) -> bool {
    let Some(add_source) = ffi::function::<AddFdSource>(c"g_unix_fd_add_full") else {
        return false;
    };
    let fd = inotify_init1(IN_CLOEXEC | IN_NONBLOCK);
    if fd < 0 {
        return false;
    }
    let mut watched = 0;
    for path in paths {
        let Some(text) = path.to_str().and_then(|p| CString::new(p).ok()) else {
            continue;
        };
        if inotify_add_watch(fd, text.as_ptr(), IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE) >= 0 {
            watched += 1;
        }
    }
    if watched == 0 {
        return false;
    }
    *HANDLER.0.get() = Some(handler);
    add_source(
        G_PRIORITY_DEFAULT,
        fd,
        G_IO_IN,
        drain,
        ptr::null_mut(),
        ptr::null(),
    );
    log!("watching {watched} directories for appearance changes");
    true
}
