use std::sync::Arc;

use tokio::sync::{broadcast, oneshot};

use crate::ffi::errors::{
    FFI_ERR_ALREADY_RUNNING, FFI_ERR_INVALID_FRAME, FFI_ERR_INVALID_UTF8, FFI_ERR_NOT_RUNNING,
    FFI_ERR_NULL_PTR, FFI_ERR_PANIC, FFI_ERR_RUNTIME, FFI_OK,
};
use crate::protocol::envelope::validate_envelope;
use crate::runtime::state::state;
use crate::{transport, Frame};

/// Start websocket server runtime for frame broadcasting
#[unsafe(no_mangle)]
pub extern "C" fn fmh_ws_start(addr_ptr: *const u8, addr_len: usize) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if addr_ptr.is_null() {
            return FFI_ERR_NULL_PTR;
        }

        // SAFETY : validated above
        let addr_bytes = unsafe { std::slice::from_raw_parts(addr_ptr, addr_len) };
        let addr = match std::str::from_utf8(addr_bytes) {
            Ok(v) => v.to_string(),
            Err(_) => return FFI_ERR_INVALID_UTF8,
        };

        let mut guard = match state().lock() {
            Ok(g) => g,
            Err(_) => return FFI_ERR_RUNTIME,
        };

        if guard.worker.is_some() {
            return FFI_ERR_ALREADY_RUNNING;
        }

        // Increase buffer（ allow burst ）
        let (frame_tx, _) = broadcast::channel::<Frame>(1024);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let worker_addr = addr.clone();
        let worker_tx = frame_tx.clone();

        let worker = std::thread::spawn(move || {
            // Threads are automatically configured based on the CPU
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
                .max(2);

            let rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(threads)
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(err) => {
                    eprintln!("Failed to build tokio runtime: {}", err);
                    return;
                }
            };

            rt.block_on(async move {
                transport::websocket_server::run_until_shutdown(
                    &worker_addr,
                    worker_tx,
                    shutdown_rx,
                )
                .await;
            });
        });

        guard.frame_tx = Some(frame_tx);
        guard.shutdown_tx = Some(shutdown_tx);
        guard.worker = Some(worker);

        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

/// Send one envelope-framed binary message to websocket subscribers
#[unsafe(no_mangle)]
pub extern "C" fn fmh_ws_send(frame_ptr: *const u8, frame_len: usize) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if frame_ptr.is_null() {
            return FFI_ERR_NULL_PTR;
        }

        // SAFETY : validated above
        let frame = unsafe { std::slice::from_raw_parts(frame_ptr, frame_len) };

        if !validate_envelope(frame) {
            return FFI_ERR_INVALID_FRAME;
        }

        // Shorten the lock range ( to avoid contention )
        let tx = {
            let guard = match state().lock() {
                Ok(g) => g,
                Err(_) => return FFI_ERR_RUNTIME,
            };

            match guard.frame_tx.as_ref() {
                Some(tx) => tx.clone(),
                None => return FFI_ERR_NOT_RUNNING,
            }
        };

        // One allocation, then Arc clone
        let frame_arc = Arc::new(frame.to_vec());

        // Send ( no-receiver allowed )
        let _ = tx.send(frame_arc);

        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}

/// Stop websocket server runtime if running
#[unsafe(no_mangle)]
pub extern "C" fn fmh_ws_stop() -> i32 {
    let result = std::panic::catch_unwind(|| {
        let mut guard = match state().lock() {
            Ok(g) => g,
            Err(_) => return FFI_ERR_RUNTIME,
        };

        if guard.worker.is_none() {
            return FFI_ERR_NOT_RUNNING;
        }

        // send shutdown signal
        if let Some(tx) = guard.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Waiting for worker to finish
        if let Some(worker) = guard.worker.take() {
            let _ = worker.join();
        }

        guard.frame_tx = None;

        FFI_OK
    });

    match result {
        Ok(code) => code,
        Err(_) => FFI_ERR_PANIC,
    }
}