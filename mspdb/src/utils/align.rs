//! Alignment and padding utilities

/// Returns the number of alignment padding bytes that are needed to reach an alignment of 4,
/// given the number of bytes already in a buffer.
pub fn alignment_bytes_needed_4(n: usize) -> usize {
    (4 - (n & 3)) & 3
}
