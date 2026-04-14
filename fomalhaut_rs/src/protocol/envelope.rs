pub const ENVELOPE_HEADER_LEN: usize = 17;
pub const ENVELOPE_VERSION_V1: u8 = 1;

pub fn validate_envelope(frame: &[u8]) -> bool {
    if frame.len() < ENVELOPE_HEADER_LEN {
        return false;
    }
    if frame[0] != ENVELOPE_VERSION_V1 {
        return false;
    }
    let payload_len = u32::from_le_bytes([frame[13], frame[14], frame[15], frame[16]]) as usize;
    payload_len == frame.len() - ENVELOPE_HEADER_LEN
}
