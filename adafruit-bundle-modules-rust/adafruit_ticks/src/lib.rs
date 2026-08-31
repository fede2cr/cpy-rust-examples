//! Rust implementation behind the CircuitPython `adafruit_ticks` native module.
//!
//! Ports the arithmetic of the bundle library at
//! `circuitpython/frozen/Adafruit_CircuitPython_Ticks/adafruit_ticks.py`.
//! `ticks_ms` is not here: it only reads the hardware clock, so the shim
//! re-exports `supervisor.ticks_ms` directly.
//!
//! The exported `extern "C"` surface is `no_std` and allocation-free so the
//! compiled object can be linked into a MicroPython native `.mpy` module.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

/// Ticks wrap at 2**29 ms so that two values can always be added or subtracted
/// without allocating a long integer.
pub const TICKS_PERIOD: i32 = 1 << 29;
pub const TICKS_MAX: i32 = TICKS_PERIOD - 1;
pub const TICKS_HALFPERIOD: i32 = TICKS_PERIOD / 2;

/// Operation completed successfully.
pub const TICKS_OK: i32 = 0;
/// The delta was outside +/- 2**28 ms.
pub const TICKS_ERR_OVERFLOW: i32 = -1;
/// A destination pointer was null.
pub const TICKS_ERR_BUFFER: i32 = -2;

/// Adds a delta to a base tick count, wrapping at `TICKS_PERIOD`.
///
/// Masking after a wrapping add matches Python's `%` here because 2**29 divides
/// 2**32, so the result is exact for every `i32` input.
#[inline]
pub fn ticks_add(ticks: i32, delta: i32) -> Option<i32> {
    if delta > -TICKS_HALFPERIOD && delta < TICKS_HALFPERIOD {
        Some(ticks.wrapping_add(delta) & TICKS_MAX)
    } else {
        None
    }
}

/// Signed difference between two tick values, assuming they are within 2**28.
// Inlined so that dependent natmods do not need this crate's object file linked in.
#[inline]
pub fn ticks_diff(ticks1: i32, ticks2: i32) -> i32 {
    let diff = ticks1.wrapping_sub(ticks2) & TICKS_MAX;
    ((diff + TICKS_HALFPERIOD) & TICKS_MAX) - TICKS_HALFPERIOD
}

/// True if `ticks1` is before `ticks2`, assuming they are within 2**28.
#[inline]
pub fn ticks_less(ticks1: i32, ticks2: i32) -> bool {
    ticks_diff(ticks1, ticks2) < 0
}

#[no_mangle]
pub unsafe extern "C" fn rust_ticks_add(ticks: i32, delta: i32, out: *mut i32) -> i32 {
    if out.is_null() {
        return TICKS_ERR_BUFFER;
    }
    match ticks_add(ticks, delta) {
        Some(value) => {
            *out = value;
            TICKS_OK
        }
        None => TICKS_ERR_OVERFLOW,
    }
}

#[no_mangle]
pub extern "C" fn rust_ticks_diff(ticks1: i32, ticks2: i32) -> i32 {
    ticks_diff(ticks1, ticks2)
}

#[no_mangle]
pub extern "C" fn rust_ticks_less(ticks1: i32, ticks2: i32) -> i32 {
    ticks_less(ticks1, ticks2) as i32
}

#[cfg(all(not(test), not(feature = "std"), feature = "panic-handler"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Transcription of the Python original, evaluated in i64 with Python's
    /// floor-modulo so it can be diffed against the i32 bit-twiddling above.
    fn py_ticks_add(ticks: i64, delta: i64) -> Option<i64> {
        if -(TICKS_HALFPERIOD as i64) < delta && delta < TICKS_HALFPERIOD as i64 {
            Some((ticks + delta).rem_euclid(TICKS_PERIOD as i64))
        } else {
            None
        }
    }

    fn py_ticks_diff(ticks1: i64, ticks2: i64) -> i64 {
        let diff = (ticks1 - ticks2) & TICKS_MAX as i64;
        ((diff + TICKS_HALFPERIOD as i64) & TICKS_MAX as i64) - TICKS_HALFPERIOD as i64
    }

    #[test]
    fn add_wraps_at_period() {
        assert_eq!(ticks_add(0, 1), Some(1));
        assert_eq!(ticks_add(TICKS_MAX, 1), Some(0));
        assert_eq!(ticks_add(0, -1), Some(TICKS_MAX));
    }

    #[test]
    fn add_rejects_out_of_range_delta() {
        assert_eq!(ticks_add(0, TICKS_HALFPERIOD), None);
        assert_eq!(ticks_add(0, -TICKS_HALFPERIOD), None);
        assert_eq!(ticks_add(0, TICKS_HALFPERIOD - 1), Some(TICKS_HALFPERIOD - 1));
    }

    #[test]
    fn diff_is_signed_and_wraps() {
        assert_eq!(ticks_diff(1, 0), 1);
        assert_eq!(ticks_diff(0, 1), -1);
        assert_eq!(ticks_diff(0, TICKS_MAX), 1);
        assert_eq!(ticks_diff(TICKS_MAX, 0), -1);
    }

    #[test]
    fn less_follows_diff() {
        assert!(ticks_less(0, 1));
        assert!(!ticks_less(1, 0));
        assert!(!ticks_less(5, 5));
        // Wrapped comparison: TICKS_MAX is "before" 0.
        assert!(ticks_less(TICKS_MAX, 0));
    }

    #[test]
    fn matches_python_reference() {
        let samples = [
            0,
            1,
            2,
            1000,
            TICKS_HALFPERIOD - 1,
            TICKS_HALFPERIOD,
            TICKS_HALFPERIOD + 1,
            TICKS_MAX - 1,
            TICKS_MAX,
        ];
        for &a in &samples {
            for &b in &samples {
                assert_eq!(
                    ticks_diff(a, b) as i64,
                    py_ticks_diff(a as i64, b as i64),
                    "ticks_diff({a}, {b})"
                );
            }
            for delta in [-1000, -1, 0, 1, 1000, TICKS_HALFPERIOD - 1, TICKS_HALFPERIOD] {
                assert_eq!(
                    ticks_add(a, delta).map(|v| v as i64),
                    py_ticks_add(a as i64, delta as i64),
                    "ticks_add({a}, {delta})"
                );
            }
        }
    }

    #[test]
    fn deadline_round_trip() {
        // The library's intended use: schedule a deadline, then test for it.
        let now = TICKS_MAX - 5;
        let deadline = ticks_add(now, 10).unwrap();
        assert!(ticks_less(now, deadline));
        assert_eq!(ticks_diff(deadline, now), 10);
    }

    #[test]
    fn c_abi_reports_overflow() {
        let mut out = 0i32;
        unsafe {
            assert_eq!(rust_ticks_add(0, TICKS_HALFPERIOD, &mut out), TICKS_ERR_OVERFLOW);
            assert_eq!(rust_ticks_add(TICKS_MAX, 1, &mut out), TICKS_OK);
        }
        assert_eq!(out, 0);
        assert_eq!(rust_ticks_less(0, 1), 1);
        assert_eq!(rust_ticks_less(1, 0), 0);
    }
}
