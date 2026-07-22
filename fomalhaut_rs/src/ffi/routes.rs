use std::ffi::c_void;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::callbacks::{HttpCallback, NativeWsCallback};
use super::errors::{
    FFI_ERR_INVALID_ROUTE, FFI_ERR_NULL_PTR, FFI_ERR_PANIC, FFI_ERR_RUNTIME, FFI_OK,
};
use crate::protocol::envelope::{ENVELOPE_HEADER_LEN, ENVELOPE_VERSION_V1};
use crate::runtime::state::{HttpRoute, WsFrame, state};

fn validate_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains('*')
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_register_http(
    method_ptr: *const u8,
    method_len: usize,
    path_ptr: *const u8,
    path_len: usize,
    callback: HttpCallback,
    userdata: *mut c_void,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if method_ptr.is_null() || path_ptr.is_null() {
            return super::errors::FFI_ERR_NULL_PTR;
        }

        let method_bytes = unsafe { std::slice::from_raw_parts(method_ptr, method_len) };
        let method = match std::str::from_utf8(method_bytes) {
            Ok(v) if !v.is_empty() => v.to_ascii_uppercase(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
        let mut path = match std::str::from_utf8(path_bytes) {
            Ok(v) if validate_path(v) => v.to_string(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        if path.len() > 1 && path.ends_with('/') {
            path.pop();
        }

        let mut guard = match state().write() {
            Ok(g) => g,
            Err(_) => return FFI_ERR_RUNTIME,
        };
        guard.http_routes.insert(
            (method.clone(), path.clone()),
            HttpRoute { callback, userdata },
        );
        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_register_post(
    path_ptr: *const u8,
    path_len: usize,
    callback: HttpCallback,
    userdata: *mut c_void,
) -> i32 {
    fmh_register_http(b"POST".as_ptr(), 4, path_ptr, path_len, callback, userdata)
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_register_websocket(path_ptr: *const u8, path_len: usize) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if path_ptr.is_null() {
            return super::errors::FFI_ERR_NULL_PTR;
        }

        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
        let path = match std::str::from_utf8(path_bytes) {
            Ok(v) if validate_path(v) => v.to_string(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        let mut guard = match state().write() {
            Ok(g) => g,
            Err(_) => return FFI_ERR_RUNTIME,
        };

        let (tx, _) = tokio::sync::watch::channel::<WsFrame>(std::sync::Arc::new(Vec::new()));
        guard.ws_routes.insert(path, tx);
        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_register_native_route(
    method_ptr: *const u8,
    method_len: usize,
    path_ptr: *const u8,
    path_len: usize,
    entity_ptr: *const u8,
    entity_len: usize,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if method_ptr.is_null() || path_ptr.is_null() || entity_ptr.is_null() {
            return super::errors::FFI_ERR_NULL_PTR;
        }

        let method_bytes = unsafe { std::slice::from_raw_parts(method_ptr, method_len) };
        let method = match std::str::from_utf8(method_bytes) {
            Ok(v) if !v.is_empty() => v.to_ascii_uppercase(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
        let mut path = match std::str::from_utf8(path_bytes) {
            Ok(v) if validate_path(v) => v.to_string(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        if path.len() > 1 && path.ends_with('/') {
            path.pop();
        }

        let entity_bytes = unsafe { std::slice::from_raw_parts(entity_ptr, entity_len) };
        let entity = match std::str::from_utf8(entity_bytes) {
            Ok(v) => v.to_string(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        let mut guard = match state().write() {
            Ok(g) => g,
            Err(_) => return FFI_ERR_RUNTIME,
        };
        guard
            .native_routes
            .insert((method.clone(), path.clone()), entity);
        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

/// Register a native Axis WebSocket stream that runs entirely inside Rust.
///
/// After registration, `fomalhaut_rs` spawns a dedicated OS thread that calls
/// `callback(userdata, &mut payload_len)` once per frame at `fps` frames per
/// second.  The returned raw bytes are immediately wrapped in a Fomalhaut v1
/// envelope and broadcast to every connected WebSocket client on `path`.
///
/// Julia is never woken up during the hot-path; this is the entry-point for
/// the `@FMHUT.axis_websocket` macro.
#[unsafe(no_mangle)]
pub extern "C" fn fmh_register_axis_ws_stream(
    path_ptr: *const u8,
    path_len: usize,
    fps: f64,
    callback: NativeWsCallback,
    userdata: *mut c_void,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if path_ptr.is_null() {
            return FFI_ERR_NULL_PTR;
        }
        if fps <= 0.0 || fps > 10_000.0 {
            return FFI_ERR_INVALID_ROUTE;
        }

        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
        let path = match std::str::from_utf8(path_bytes) {
            Ok(v) if validate_path(v) => v.to_string(),
            _ => return FFI_ERR_INVALID_ROUTE,
        };

        // Create a watch channel — the sender goes into ServerState, the
        // receiver is cloned by each WebSocket connection handler.
        let (tx, _) = tokio::sync::watch::channel::<WsFrame>(Arc::new(Vec::new()));

        {
            let mut guard = match state().write() {
                Ok(g) => g,
                Err(_) => return FFI_ERR_RUNTIME,
            };
            guard.axis_ws_routes.insert(path.clone(), tx.clone());
        }

        // `userdata` may be a raw pointer into Julia-managed memory.
        // We transmute it to usize so it can cross the thread boundary without
        // a Send bound.  Safety : the caller guarantees the memory outlives the
        // server's lifetime.
        let userdata_addr = userdata as usize;
        let interval = Duration::from_secs_f64(1.0 / fps);

        std::thread::Builder::new()
            .name(format!("fmhut-axis-ws:{}", path))
            .spawn(move || {
                // Re-constitute the userdata pointer inside the thread.
                let userdata = userdata_addr as *mut c_void;

                loop {
                    let frame_start = std::time::Instant::now();

                    let mut payload_len: usize = 0;
                    let payload_ptr = unsafe { callback(userdata, &mut payload_len as *mut usize) };

                    // Skip this frame if the callback signals "no data".
                    if !payload_ptr.is_null() && payload_len > 0 {
                        // Build the 17-byte v1 envelope inline — no heap
                        // allocation beyond the final Vec.
                        let timestamp_ns = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64;

                        // content_type 1 = FLOAT32_TENSOR ( matches Julia side )
                        let content_type: u16 = 1;
                        let flags: u16 = 0;

                        let mut frame: Vec<u8> =
                            Vec::with_capacity(ENVELOPE_HEADER_LEN + payload_len);

                        frame.push(ENVELOPE_VERSION_V1);
                        frame.extend_from_slice(&content_type.to_le_bytes());
                        frame.extend_from_slice(&flags.to_le_bytes());
                        frame.extend_from_slice(&timestamp_ns.to_le_bytes());
                        frame.extend_from_slice(&(payload_len as u32).to_le_bytes());

                        // SAFETY: callback guarantees the slice is valid for
                        // at least this copy.
                        let payload_slice =
                            unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) };
                        frame.extend_from_slice(payload_slice);

                        // Broadcast; ignore send errors ( no receivers yet is fine ).
                        let _ = tx.send(Arc::new(frame));
                    }

                    // Busy-wait / sleep to honour the target frame interval.
                    let elapsed = frame_start.elapsed();
                    if elapsed < interval {
                        std::thread::sleep(interval - elapsed);
                    }
                }
            })
            .map_err(|_| FFI_ERR_RUNTIME)
            .map(|_| FFI_OK)
            .unwrap_or(FFI_ERR_RUNTIME)
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_malloc(size: usize) -> *mut u8 {
    unsafe { libc::malloc(size) as *mut u8 }
}

#[unsafe(no_mangle)]
pub extern "C" fn fmh_free(ptr: *mut u8) {
    if !ptr.is_null() {
        unsafe { libc::free(ptr.cast()) };
    }
}
