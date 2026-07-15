use tokio::sync::oneshot;

use crate::runtime::state::HttpRoute;

pub type HttpCallback = Option<unsafe extern "C" fn(*mut std::ffi::c_void)>;

/// C-compatible callback for native Axis WebSocket frame generation.
///
/// Invoked by `fomalhaut_rs` per frame from a dedicated OS thread.
///
/// # Safety
/// The returned pointer must remain valid and immutable until the next invocation.
/// The caller ( `fomalhaut_rs` ) copies the payload immediately and does not take ownership.
///
/// # Parameters
/// * `userdata` - User-provided context pointer registered with the callback.
/// * `out_len`  - Output parameter populated with the payload size in bytes.
///
/// # Returns
/// A pointer to the raw payload ( e.g., GPU readback buffer ). 
/// Returns `null` or writes `0` to `out_len` to skip the current frame.
pub type NativeWsCallback =
    unsafe extern "C" fn(userdata: *mut std::ffi::c_void, out_len: *mut usize) -> *const u8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiHttpResponse {
    pub body_ptr: *mut u8,
    pub body_len: usize,
    pub content_type_ptr: *mut u8,
    pub content_type_len: usize,
    pub status_code: u16,
}

pub struct CallbackResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub content_type: String,
}

pub struct HttpTask {
    pub route: HttpRoute,
    pub method: Vec<u8>,
    pub path: Vec<u8>,
    pub query: Vec<u8>,
    pub headers: Vec<u8>,
    pub body: Vec<u8>,
    pub response_tx: oneshot::Sender<Result<CallbackResponse, i32>>,
}

/// The task handle used by Julia is heap-allocated; Julia calls `fmh_complete_http_task` to release it after use
pub struct FfiHttpTaskHandle {
    pub method: Vec<u8>,
    pub path: Vec<u8>,
    pub query: Vec<u8>,
    pub headers: Vec<u8>,
    pub body: Vec<u8>,
    pub route: HttpRoute,
    pub response_tx: tokio::sync::oneshot::Sender<Result<CallbackResponse, i32>>,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FfiHttpTaskData {
    pub method_ptr: *const u8,
    pub method_len: usize,
    pub path_ptr: *const u8,
    pub path_len: usize,
    pub query_ptr: *const u8,
    pub query_len: usize,
    pub headers_ptr: *const u8,
    pub headers_len: usize,
    pub body_ptr: *const u8,
    pub body_len: usize,
    pub task_handle: *mut FfiHttpTaskHandle,
}
