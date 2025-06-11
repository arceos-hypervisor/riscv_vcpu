
pub const EID_DBCN: usize = 0x4442434e;
pub const FID_CONSOLE_WRITE: usize = 0;
pub const FID_CONSOLE_READ: usize = 1;
pub const FID_CONSOLE_WRITE_BYTE: usize = 2;


/// SBI success state return value.
pub const RET_SUCCESS: usize = 0;
/// Error for SBI call failed for unknown reasons.
pub const RET_ERR_FAILED: usize = -1isize as _;
/// Error for target operation not supported.
pub const RET_ERR_NOT_SUPPORTED: usize = -2isize as _;

/// Writes a full string to console using SBI byte-wise API (no log prefix).
#[inline(always)]
pub fn print_raw(s: &str) {
    for &b in s.as_bytes() {
        sbi_rt::console_write_byte(b);
    }
}

/// Writes a full string + newline to console (no log prefix).
#[inline(always)]
pub fn print_rawln(s: &str) {
    print_raw(s);
    sbi_rt::console_write_byte(b'\n');
}

/// Writes a byte to the console.
#[inline(always)]
pub fn putchar(c: u8) {
    sbi_rt::console_write_byte(c);
}

/// Joins two `usize` values into a `u64` value representing a guest physical address (GPA).
#[inline(always)]
pub fn join_u64(base_lo: usize, base_hi: usize) -> u64 {
    ((base_hi as u64) << 32) | (base_lo as u64)
}