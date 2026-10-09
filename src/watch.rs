// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::path::Path;
use std::ptr;

use crate::gtk::{AddFdSource, GBoolean, TimeoutAdd};
use crate::log;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOSE_WRITE: u32 = 0x0000_0008;
const IN_MOVED_TO: u32 = 0x0000_0080;
const IN_CREATE: u32 = 0x0000_0100;
const G_IO_IN: c_uint = 1;
const G_PRIORITY_DEFAULT: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;
const G_SOURCE_REMOVE: GBoolean = 0;
const SETTLE_MILLISECONDS: c_uint = 150;

extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

struct Pending {
    handler: Option<fn()>,
    timeout_add: Option<TimeoutAdd>,
    scheduled: bool,
}

struct State(UnsafeCell<Pending>);

unsafe impl Sync for State {}

static STATE: State = State(UnsafeCell::new(Pending {
    handler: None,
    timeout_add: None,
    scheduled: false,
}));

extern "C" fn drain(fd: c_int, _condition: c_uint, _data: *mut c_void) -> c_int {
    let mut buffer = [0u8; 4096];
    unsafe {
        while read(fd, buffer.as_mut_ptr().cast(), buffer.len()) > 0 {}
        let state = &mut *STATE.0.get();
        if let (false, Some(timeout_add)) = (state.scheduled, state.timeout_add) {
            state.scheduled = true;
            timeout_add(SETTLE_MILLISECONDS, settle, ptr::null_mut());
        }
    }
    G_SOURCE_CONTINUE
}

extern "C" fn settle(_data: *mut c_void) -> GBoolean {
    unsafe {
        let state = &mut *STATE.0.get();
        state.scheduled = false;
        if let Some(handler) = state.handler {
            handler();
        }
    }
    G_SOURCE_REMOVE
}

pub unsafe fn directories(
    paths: &[&Path],
    add_source: AddFdSource,
    timeout_add: TimeoutAdd,
    handler: fn(),
) -> bool {
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
        close(fd);
        return false;
    }
    let state = &mut *STATE.0.get();
    state.handler = Some(handler);
    state.timeout_add = Some(timeout_add);
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
