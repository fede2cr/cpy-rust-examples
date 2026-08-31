//! Rust implementation behind the CircuitPython `hello` native module.
//!
//! The exported `extern "C"` surface is `no_std` and allocation-free so the
//! compiled object can be linked into a MicroPython native `.mpy` module.

#![cfg_attr(not(test), no_std)]

use core::slice;

const GREETING_PREFIX: &[u8] = b"Hello, ";
const GREETING_SUFFIX: &[u8] = b" from Rust!";

/// Operation completed successfully.
pub const HELLO_OK: i32 = 0;
/// The requested operation name is not supported.
pub const HELLO_ERR_UNSUPPORTED: i32 = -1;
/// Division by zero was requested.
pub const HELLO_ERR_DIV_ZERO: i32 = -2;
/// A destination buffer was too small or a pointer was invalid.
pub const HELLO_ERR_BUFFER: i32 = -3;

pub fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

pub fn sub(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

pub fn mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

pub fn div(a: i32, b: i32) -> Option<f32> {
    if b == 0 {
        None
    } else {
        Some(a as f32 / b as f32)
    }
}

pub fn calc(op: &[u8], a: i32, b: i32) -> Option<i32> {
    match op {
        b"add" | b"+" => Some(add(a, b)),
        b"sub" | b"-" => Some(sub(a, b)),
        b"mul" | b"*" => Some(mul(a, b)),
        _ => None,
    }
}

/// Writes `Hello, <name> from Rust!` into `out`, returning the byte length.
///
/// Written with checked indexing and plain loops so the generated code has no
/// panic paths and no references to out-of-line `core` helpers; neither can be
/// relocated into a native .mpy.
pub fn greet_into(name: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut at = copy_part(GREETING_PREFIX, out, 0)?;
    at = copy_part(name, out, at)?;
    copy_part(GREETING_SUFFIX, out, at)
}

fn copy_part(src: &[u8], out: &mut [u8], mut at: usize) -> Option<usize> {
    let mut i = 0;
    while i < src.len() {
        *out.get_mut(at)? = *src.get(i)?;
        at += 1;
        i += 1;
    }
    Some(at)
}

/// Treats a null pointer as an empty slice rather than dereferencing it.
unsafe fn as_slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        slice::from_raw_parts(ptr, len)
    }
}

#[no_mangle]
pub extern "C" fn rust_hello_add(a: i32, b: i32) -> i32 {
    add(a, b)
}

#[no_mangle]
pub extern "C" fn rust_hello_sub(a: i32, b: i32) -> i32 {
    sub(a, b)
}

#[no_mangle]
pub extern "C" fn rust_hello_mul(a: i32, b: i32) -> i32 {
    mul(a, b)
}

#[no_mangle]
pub unsafe extern "C" fn rust_hello_div(a: i32, b: i32, out: *mut f32) -> i32 {
    if out.is_null() {
        return HELLO_ERR_BUFFER;
    }
    match div(a, b) {
        Some(value) => {
            *out = value;
            HELLO_OK
        }
        None => HELLO_ERR_DIV_ZERO,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_hello_calc(
    op: *const u8,
    op_len: usize,
    a: i32,
    b: i32,
    out: *mut i32,
) -> i32 {
    if out.is_null() {
        return HELLO_ERR_BUFFER;
    }
    match calc(as_slice(op, op_len), a, b) {
        Some(value) => {
            *out = value;
            HELLO_OK
        }
        None => HELLO_ERR_UNSUPPORTED,
    }
}

/// Returns the number of bytes written, or a negative `HELLO_ERR_*` code.
#[no_mangle]
pub unsafe extern "C" fn rust_hello_greet(
    name: *const u8,
    name_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> i32 {
    if out.is_null() || out_cap == 0 {
        return HELLO_ERR_BUFFER;
    }
    let name = as_slice(name, name_len);
    let buffer = slice::from_raw_parts_mut(out, out_cap);
    match greet_into(name, buffer) {
        Some(written) => written as i32,
        None => HELLO_ERR_BUFFER,
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_works() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(sub(10, 4), 6);
        assert_eq!(mul(7, 6), 42);
        assert_eq!(div(9, 3), Some(3.0));
        assert_eq!(div(1, 0), None);
    }

    #[test]
    fn calc_supports_names_and_symbols() {
        assert_eq!(calc(b"add", 2, 3), Some(5));
        assert_eq!(calc(b"*", 7, 6), Some(42));
        assert_eq!(calc(b"pow", 2, 3), None);
    }

    #[test]
    fn greet_writes_expected_bytes() {
        let mut buf = [0u8; 64];
        let n = greet_into(b"ESP32-S3", &mut buf).unwrap();
        assert_eq!(&buf[..n], b"Hello, ESP32-S3 from Rust!");
    }

    #[test]
    fn greet_rejects_small_buffer() {
        let mut buf = [0u8; 4];
        assert_eq!(greet_into(b"ESP32-S3", &mut buf), None);
    }

    #[test]
    fn c_abi_reports_division_by_zero() {
        let mut out = 0.0f32;
        unsafe {
            assert_eq!(rust_hello_div(1, 0, &mut out), HELLO_ERR_DIV_ZERO);
            assert_eq!(rust_hello_div(9, 3, &mut out), HELLO_OK);
        }
        assert_eq!(out, 3.0);
    }
}
