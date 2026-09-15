//! Fixed-point Mandelbrot kernel, the workload the turbo project uses to compare
//! CircuitPython bytecode, `@micropython.native` and `@micropython.viper`.
//!
//! Ported from `examples/mandelbrot/src/pixels.py` of <https://github.com/mikeysklar/turbo>
//! (MIT). The numerics are pinned to the viper version: 12 fractional bits and
//! wrapping 32-bit signed arithmetic, so 160x120 at 64 iterations returns
//! 407644. A variant with a different checksum is a different program, not a
//! faster one.
//!
//! The exported `extern "C"` surface is `no_std` and allocation-free so the
//! compiled object can be linked into a MicroPython native `.mpy` module.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

/// Fractional bits in the fixed-point representation.
pub const FRAC_BITS: u32 = 12;
/// 1.0 in fixed point.
pub const ONE: i32 = 1 << FRAC_BITS;
/// Escape threshold, |z|^2 > 4.0.
pub const ESCAPE: i32 = 4 << FRAC_BITS;

/// Iteration counts for one scanline, written as bytes exactly like viper's
/// `ptr8` store, which truncates.
///
/// `dx` is the fixed-point step between columns and `cy` the row's imaginary
/// part. Every operation wraps at 32 bits to match the viper emitter, whose
/// `int` is a raw machine word with no overflow promotion to a Python long.
pub fn mandel_row(out: &mut [u8], dx: i32, cy: i32, max_iter: i32) {
    for (px, cell) in out.iter_mut().enumerate() {
        let cx = (px as i32).wrapping_mul(dx).wrapping_sub(2 << FRAC_BITS);
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut i: i32 = 0;
        while i < max_iter {
            let x2 = x.wrapping_mul(x) >> FRAC_BITS;
            let y2 = y.wrapping_mul(y) >> FRAC_BITS;
            if x2.wrapping_add(y2) > ESCAPE {
                break;
            }
            y = (x.wrapping_mul(y) >> (FRAC_BITS - 1)).wrapping_add(cy);
            x = x2.wrapping_sub(y2).wrapping_add(cx);
            i += 1;
        }
        *cell = i as u8;
    }
}

/// Sum of every pixel's iteration count over the whole image.
///
/// Same arithmetic as [`mandel_row`], but the row loop and the accumulation
/// stay on this side of the boundary instead of running as Python.
pub fn mandel_sum(width: i32, height: i32, max_iter: i32) -> u32 {
    if width <= 0 || height <= 0 {
        return 0;
    }
    let dx = (3 << FRAC_BITS) / width;
    let mut total: u32 = 0;
    for r in 0..height {
        let cy = ((r * 2) << FRAC_BITS) / height - ONE;
        for px in 0..width {
            let cx = px.wrapping_mul(dx).wrapping_sub(2 << FRAC_BITS);
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut i: i32 = 0;
            while i < max_iter {
                let x2 = x.wrapping_mul(x) >> FRAC_BITS;
                let y2 = y.wrapping_mul(y) >> FRAC_BITS;
                if x2.wrapping_add(y2) > ESCAPE {
                    break;
                }
                y = (x.wrapping_mul(y) >> (FRAC_BITS - 1)).wrapping_add(cy);
                x = x2.wrapping_sub(y2).wrapping_add(cx);
                i += 1;
            }
            // Truncated to a byte first, because the Python harness sums a bytearray.
            total = total.wrapping_add(i as u8 as u32);
        }
    }
    total
}

/// # Safety
/// `out` must point to at least `width` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn rust_mandel_row(
    out: *mut u8,
    width: i32,
    dx: i32,
    cy: i32,
    max_iter: i32,
) {
    if out.is_null() || width <= 0 {
        return;
    }
    mandel_row(
        core::slice::from_raw_parts_mut(out, width as usize),
        dx,
        cy,
        max_iter,
    );
}

#[no_mangle]
pub extern "C" fn rust_mandel_sum(width: i32, height: i32, max_iter: i32) -> u32 {
    mandel_sum(width, height, max_iter)
}

#[cfg(all(not(test), not(feature = "std"), feature = "panic-handler"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 160;
    const H: i32 = 120;
    const IT: i32 = 64;
    /// The value every turbo variant returns on every board it was measured on.
    const EXPECTED: u32 = 407_644;

    fn sweep_rows() -> u32 {
        let mut row = [0u8; W as usize];
        let dx = (3 << FRAC_BITS) / W;
        let mut total: u32 = 0;
        for r in 0..H {
            mandel_row(&mut row, dx, ((r * 2) << FRAC_BITS) / H - ONE, IT);
            total += row.iter().map(|&b| b as u32).sum::<u32>();
        }
        total
    }

    #[test]
    fn row_kernel_matches_the_published_checksum() {
        assert_eq!(sweep_rows(), EXPECTED);
    }

    #[test]
    fn whole_image_matches_the_row_kernel() {
        assert_eq!(mandel_sum(W, H, IT), EXPECTED);
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        assert_eq!(mandel_sum(0, H, IT), 0);
        assert_eq!(mandel_sum(W, 0, IT), 0);
        assert_eq!(mandel_sum(W, H, 0), 0);
    }
}
