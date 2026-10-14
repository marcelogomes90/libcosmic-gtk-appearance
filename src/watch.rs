// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 Marcelo

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::ptr;

use crate::gtk::{AddFdSource, GBoolean, TimeoutAdd};
use crate::log;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOSE_WRITE: u32 = 0x0000_0008;
const IN_MOVED_FROM: u32 = 0x0000_0040;
const IN_MOVED_TO: u32 = 0x0000_0080;
const IN_CREATE: u32 = 0x0000_0100;
const IN_DELETE: u32 = 0x0000_0200;
const IN_DELETE_SELF: u32 = 0x0000_0400;
const IN_MOVE_SELF: u32 = 0x0000_0800;
const IN_ATTRIB: u32 = 0x0000_0004;
const IN_IGNORED: u32 = 0x0000_8000;
const WATCH_MASK: u32 = IN_CLOSE_WRITE
    | IN_MOVED_FROM
    | IN_MOVED_TO
    | IN_CREATE
    | IN_DELETE
    | IN_DELETE_SELF
    | IN_MOVE_SELF
    | IN_ATTRIB;
const LOST_WATCH: u32 = IN_IGNORED | IN_DELETE_SELF | IN_MOVE_SELF;
const EVENT_HEADER: usize = 16;
const G_IO_IN: c_uint = 1;
const G_IO_BROKEN: c_uint = 8 | 16 | 32;
const G_PRIORITY_DEFAULT: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;
const G_SOURCE_DROP: c_int = 0;
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
    fd: c_int,
    paths: Vec<CString>,
}

struct State(UnsafeCell<Pending>);

unsafe impl Sync for State {}

static STATE: State = State(UnsafeCell::new(Pending {
    handler: None,
    timeout_add: None,
    scheduled: false,
    fd: -1,
    paths: Vec::new(),
}));

unsafe fn arm(fd: c_int, paths: &[CString]) -> usize {
    paths
        .iter()
        .filter(|path| inotify_add_watch(fd, path.as_ptr(), WATCH_MASK) >= 0)
        .count()
}

fn field_at(buffer: &[u8], offset: usize) -> u32 {
    buffer
        .get(offset..offset + 4)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map_or(0, u32::from_ne_bytes)
}

unsafe fn drain_events(fd: c_int) -> bool {
    let mut buffer = [0u8; 4096];
    let mut lost = false;
    loop {
        let read_bytes = read(fd, buffer.as_mut_ptr().cast(), buffer.len());
        if read_bytes <= 0 {
            return lost;
        }
        let filled = read_bytes as usize;
        let mut offset = 0usize;
        while offset + EVENT_HEADER <= filled {
            let mask = field_at(&buffer, offset + 4);
            let name_length = field_at(&buffer, offset + 12) as usize;
            if mask & LOST_WATCH != 0 {
                lost = true;
            }
            let Some(next) = offset
                .checked_add(EVENT_HEADER)
                .and_then(|past_header| past_header.checked_add(name_length))
            else {
                break;
            };
            if next <= offset {
                break;
            }
            offset = next;
        }
    }
}

extern "C" fn drain(fd: c_int, condition: c_uint, _data: *mut c_void) -> c_int {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let state = &mut *STATE.0.get();
        if condition & G_IO_BROKEN != 0 {
            log!("the appearance watch broke, giving up on live updates");
            close(fd);
            state.fd = -1;
            return G_SOURCE_DROP;
        }
        if drain_events(fd) {
            let rearmed = arm(fd, &state.paths);
            log!("a watched directory was replaced, re-armed {rearmed} of them");
        }
        if let (false, Some(timeout_add)) = (state.scheduled, state.timeout_add) {
            state.scheduled = true;
            timeout_add(SETTLE_MILLISECONDS, settle, ptr::null_mut());
        }
        G_SOURCE_CONTINUE
    }))
    .unwrap_or(G_SOURCE_CONTINUE)
}

extern "C" fn settle(_data: *mut c_void) -> GBoolean {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        let state = &mut *STATE.0.get();
        state.scheduled = false;
        if let Some(handler) = state.handler {
            handler();
        }
    }));
    G_SOURCE_REMOVE
}

pub unsafe fn directories(
    paths: &[&Path],
    add_source: AddFdSource,
    timeout_add: TimeoutAdd,
    handler: fn(),
) -> bool {
    let state = &mut *STATE.0.get();
    let fresh = state.fd < 0;
    if fresh {
        let fd = inotify_init1(IN_CLOEXEC | IN_NONBLOCK);
        if fd < 0 {
            return false;
        }
        state.fd = fd;
    }
    state.paths = paths
        .iter()
        .filter_map(|path| path.to_str().and_then(|text| CString::new(text).ok()))
        .collect();
    let watched = arm(state.fd, &state.paths);
    if watched == 0 {
        if fresh {
            close(state.fd);
            state.fd = -1;
        }
        return false;
    }
    state.handler = Some(handler);
    state.timeout_add = Some(timeout_add);
    if fresh {
        let source = add_source(
            G_PRIORITY_DEFAULT,
            state.fd,
            G_IO_IN,
            drain,
            ptr::null_mut(),
            ptr::null(),
        );
        if source == 0 {
            log!("could not watch the appearance files");
            close(state.fd);
            state.fd = -1;
            return false;
        }
    }
    log!("watching {watched} directories for appearance changes");
    true
}
