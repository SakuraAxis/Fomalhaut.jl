use std::sync::Arc;
use tokio::sync::broadcast;

pub mod ffi;
pub mod protocol;
pub mod runtime;
pub mod transport;

pub type Frame = Arc<Vec<u8>>;
pub type FrameSender = broadcast::Sender<Frame>;
pub use ffi::{fmh_ws_send, fmh_ws_start, fmh_ws_stop};

#[cfg(test)]
mod tests {
    use super::ffi::errors::{FFI_ERR_ALREADY_RUNNING, FFI_ERR_NOT_RUNNING, FFI_OK};
    use super::protocol::envelope::{validate_envelope, ENVELOPE_HEADER_LEN, ENVELOPE_VERSION_V1};
    use super::{fmh_ws_send, fmh_ws_start, fmh_ws_stop};

    fn build_test_frame(payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(ENVELOPE_HEADER_LEN + payload.len());
        frame.push(ENVELOPE_VERSION_V1);
        frame.extend_from_slice(&1u16.to_le_bytes());
        frame.extend_from_slice(&0u16.to_le_bytes());
        frame.extend_from_slice(&123u64.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    #[test]
    fn envelope_validation_works() {
        let frame = build_test_frame(&[1, 2, 3]);
        assert!(validate_envelope(&frame));
        assert!(!validate_envelope(&[0, 1, 2]));
    }

    #[test]
    fn lifecycle_start_send_stop_works() {
        let addr = b"127.0.0.1:19091";
        assert_eq!(fmh_ws_stop(), FFI_ERR_NOT_RUNNING);
        assert_eq!(fmh_ws_start(addr.as_ptr(), addr.len()), FFI_OK);
        assert_eq!(fmh_ws_start(addr.as_ptr(), addr.len()), FFI_ERR_ALREADY_RUNNING);

        let frame = build_test_frame(&[9, 8, 7, 6]);
        assert_eq!(fmh_ws_send(frame.as_ptr(), frame.len()), FFI_OK);
        assert_eq!(fmh_ws_stop(), FFI_OK);
        assert_eq!(fmh_ws_stop(), FFI_ERR_NOT_RUNNING);
    }
}
