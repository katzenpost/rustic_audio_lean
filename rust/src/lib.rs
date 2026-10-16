// SPDX-FileCopyrightText: Copyright (C) 2026 David Stainton
// SPDX-License-Identifier: AGPL-3.0-only

//! Rustic Audio's voice notes for Lean: recording a clip to an Ogg Opus file and playing one,
//! as functions with Lean's calling convention. `RusticAudio.lean` declares them.

#![allow(clippy::missing_safety_doc)]

use lean_sys::*;
use rustic_audio::VoiceNoteEngine;
use std::sync::Mutex;

/// Changes whenever a declaration in `RusticAudio.lean` must change with this file.
pub const ABI_VERSION: u32 = 1;

unsafe fn string_of(s: b_lean_obj_arg) -> String {
    let bytes = std::slice::from_raw_parts(lean_string_cstr(s) as *const u8, lean_string_size(s) - 1);
    String::from_utf8_lossy(bytes).into_owned()
}

unsafe fn lean_string(s: &str) -> lean_obj_res {
    lean_mk_string_from_bytes(s.as_ptr(), s.len())
}

unsafe fn io_ok(v: lean_obj_res) -> lean_obj_res {
    lean_io_result_mk_ok(v)
}

unsafe fn io_err(msg: &str) -> lean_obj_res {
    lean_io_result_mk_error(lean_mk_io_user_error(lean_string(msg)))
}

unsafe fn io_unit(r: Result<(), String>) -> lean_obj_res {
    match r {
        Ok(()) => io_ok(lean_box(0)),
        Err(e) => io_err(&e),
    }
}

/// The class of Lean's handles on engines: finalizing one cancels its recording and playback.
fn engine_class() -> *mut lean_external_class {
    static CLASS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    unsafe extern "C" fn finalize(p: *mut std::ffi::c_void) {
        drop(Box::from_raw(p as *mut Mutex<VoiceNoteEngine>));
    }
    unsafe extern "C" fn foreach(_: *mut std::ffi::c_void, _: b_lean_obj_arg) {}
    *CLASS.get_or_init(|| unsafe {
        lean_register_external_class(Some(finalize), Some(foreach)) as usize
    }) as *mut lean_external_class
}

unsafe fn with_engine<T>(
    e: b_lean_obj_arg,
    f: impl FnOnce(&mut VoiceNoteEngine) -> Result<T, String>,
) -> Result<T, String> {
    let m = &*(lean_get_external_data(e) as *const Mutex<VoiceNoteEngine>);
    let mut g = m.lock().map_err(|_| "audio engine mutex poisoned".to_string())?;
    f(&mut g)
}

#[no_mangle]
pub extern "C" fn rustic_audio_abi_version(_: lean_obj_arg) -> u32 {
    ABI_VERSION
}

/// An engine whose recordings go in `dir`.
#[no_mangle]
pub unsafe extern "C" fn ra_engine_new(dir: b_lean_obj_arg) -> lean_obj_res {
    match VoiceNoteEngine::new(string_of(dir)) {
        Ok(e) => io_ok(lean_alloc_external(engine_class(), Box::into_raw(Box::new(Mutex::new(e))) as *mut _)),
        Err(e) => io_err(&e),
    }
}

/// Starts recording; the path the clip will have.
#[no_mangle]
pub unsafe extern "C" fn ra_start_capture(e: b_lean_obj_arg, stem: b_lean_obj_arg) -> lean_obj_res {
    let stem = if lean_is_scalar(stem) { None } else { Some(string_of(lean_ctor_get(stem, 0))) };
    match with_engine(e, |e| e.start_capture(stem.as_deref())) {
        Ok(p) => io_ok(lean_string(&p.to_string_lossy())),
        Err(e) => io_err(&e),
    }
}

/// Stops recording and encodes the clip: its path, milliseconds and bytes.
#[no_mangle]
pub unsafe extern "C" fn ra_stop_capture(e: b_lean_obj_arg) -> lean_obj_res {
    match with_engine(e, |e| e.stop_capture()) {
        Ok(c) => {
            let o = lean_alloc_ctor(0, 3, 0);
            lean_ctor_set(o, 0, lean_string(&c.path().to_string_lossy()));
            lean_ctor_set(o, 1, lean_uint64_to_nat((c.duration_seconds() * 1000.0).round() as u64));
            lean_ctor_set(o, 2, lean_uint64_to_nat(c.file_size_bytes()));
            io_ok(o)
        }
        Err(e) => io_err(&e),
    }
}

/// Stops recording and throws the clip away; whether one was being recorded.
#[no_mangle]
pub unsafe extern "C" fn ra_cancel_capture(e: b_lean_obj_arg) -> lean_obj_res {
    match with_engine(e, |e| e.cancel_capture()) {
        Ok(b) => io_ok(lean_box(b as usize)),
        Err(e) => io_err(&e),
    }
}

/// Plays an Ogg Opus file, stopping whatever was playing.
#[no_mangle]
pub unsafe extern "C" fn ra_play(e: b_lean_obj_arg, path: b_lean_obj_arg) -> lean_obj_res {
    let path = string_of(path);
    io_unit(with_engine(e, |e| e.play_received(path)))
}

#[no_mangle]
pub unsafe extern "C" fn ra_stop_playback(e: b_lean_obj_arg) -> lean_obj_res {
    io_unit(with_engine(e, |e| e.stop_playback()))
}

#[no_mangle]
pub unsafe extern "C" fn ra_is_recording(e: b_lean_obj_arg) -> lean_obj_res {
    match with_engine(e, |e| Ok(e.is_recording())) {
        Ok(b) => io_ok(lean_box(b as usize)),
        Err(e) => io_err(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ra_is_playing(e: b_lean_obj_arg) -> lean_obj_res {
    match with_engine(e, |e| Ok(e.is_playing())) {
        Ok(b) => io_ok(lean_box(b as usize)),
        Err(e) => io_err(&e),
    }
}

/// Why playback stopped early, once.
#[no_mangle]
pub unsafe extern "C" fn ra_take_playback_error(e: b_lean_obj_arg) -> lean_obj_res {
    match with_engine(e, |e| Ok(e.take_playback_error())) {
        Ok(None) => io_ok(lean_box(0)),
        Ok(Some(m)) => {
            let o = lean_alloc_ctor(1, 1, 0);
            lean_ctor_set(o, 0, lean_string(&m));
            io_ok(o)
        }
        Err(e) => io_err(&e),
    }
}
