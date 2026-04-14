use crate::ffi::errors::{FFI_ERR_NULL_PTR, FFI_ERR_PANIC, FFI_OK};

/// Process wave data in-place; ownership stays in Julia
#[unsafe(no_mangle)]
pub extern "C" fn process_wave_data(ptr: *mut f32, len: usize) -> i32 {
    // Never dereference a null pointer from FFI
    if ptr.is_null() {
        return FFI_ERR_NULL_PTR;
    }

    // Convert Rust panic into status code, do not unwind across FFI
    let result = std::panic::catch_unwind(|| {
        // SAFETY : pointer nullability is checked above; caller guarantees `len` valid elements
        // Rust only borrows the caller-provided buffer and never frees it
        let data = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        for value in data.iter_mut() {
            *value *= 2.0;
        }
    });

    if result.is_err() {
        return FFI_ERR_PANIC;
    }

    FFI_OK
}
