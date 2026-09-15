# `pixels_rs` — a Mandelbrot kernel in Rust

A Rust port of the one function the
[turbo](https://github.com/mikeysklar/turbo) project compiles with
`@micropython.viper`, built as a native `.mpy` named `pixels_rs`.

This module exists to answer a narrow question: **what does Rust buy you over
`@micropython.viper`, on the same board, for the same arithmetic?** Its
`mandel_row` takes the same arguments and returns the same bytes as the viper
function, so both can be imported and timed in a single session.

Unlike the other modules here, this one is built for **two** architectures,
because the interesting comparison is how the gap changes across chips.

## The checksum is the contract

The numerics are pinned to the viper version — 12 fractional bits, wrapping
32-bit arithmetic — so 160x120 at 64 iterations must return **407644**.

That value is checked twice: by the host unit tests, and again on the board
before any timing starts. This matters more than it looks. Most of the ways to
make this kernel faster (floating point, early bailout, a wider fixed-point
format) also change what it computes, and a variant with a different checksum is
a different program, not a faster one.

```bash
make -C natmod rust-tests
```

`--features std` is what switches the crate over for a host test run; the
default build is `no_std`.

## API

```python
import pixels_rs

buf = bytearray(width)
pixels_rs.mandel_row(buf, width, dx, cy, max_iter)   # fills buf with one row
total = pixels_rs.mandel_sum(width, height, max_iter)  # whole image, returns the checksum
total = pixels_rs._turbo_bench()                     # mandel_sum(160, 120, 64)
```

`mandel_row` is signature-compatible with turbo's viper function of the same
name, which is what makes the comparison fair — the module can even be renamed
to `pixels` to drop in over it. It does add one bounds check that viper's `ptr8`
does not: a `width` longer than the buffer raises `ValueError` instead of
writing past the end. That costs one comparison per row, not per pixel.

`mandel_sum` keeps the row loop on the Rust side and returns only the summed
total, so the interpreter boundary is crossed once per image instead of once per
row. That is the upper bound on what this approach can buy you.

## Getting the module

Prebuilt `.mpy` files are attached to each [release](../../../releases), one zip
per architecture. Pick the one matching your board:

| Board | Architecture | Port |
| --- | --- | --- |
| QT Py ESP32-S3 | `xtensawin` | `espressif` |
| Metro M7 1011 | `armv7emsp` | `mimxrt10xx` |
| Metro M4 Express | `armv7emsp` | `atmel-samd` |
| muzi Base Duo (nRF52840) | `armv7emsp` | `nordic` |

Copy the `.mpy` to `CIRCUITPY`, at the top level or in `lib/`, keeping the
filename. A native `.mpy` is tied to both the architecture and the .mpy format
version; the wrong one fails the import rather than misbehaving quietly.

## Building it yourself

```bash
. ~/export-esp.sh                       # xtensawin only
CPY=/path/to/circuitpython
PY=/path/to/venv/bin/python             # needs pyelftools and ar

make -C natmod ARCH=xtensawin MPY_DIR=$CPY PYTHON=$PY
make -C natmod ARCH=armv7emsp MPY_DIR=$CPY PYTHON=$PY
```

`ARCH` defaults to `xtensawin`. `armv7emsp` needs a nightly Rust with the
`rust-src` component (for `-Zbuild-std`) and `arm-none-eabi-gcc`; it does not
need the `esp` toolchain. See the [repository README](../README.md) for what
`MPY_DIR` has to point at and why a fork is needed.

`make -C natmod dist` copies the result into a per-architecture `DIST`
directory, which is overridable:

```bash
make -C natmod dist ARCH=armv7emsp DIST=/path/to/somewhere MPY_DIR=$CPY PYTHON=$PY
```

## The firmware has to allow it

Stock CircuitPython **refuses to import a native `.mpy`**. The firmware must be
built with `CIRCUITPY_ENABLE_MPY_NATIVE=1`, which turns on the Xtensa or Thumb
native emitter and is off by default on every port.

That flag is not free, and on one of these boards it is not even easy:

- The **Metro M7** and the **ESP32-S3** absorb it with room to spare.
- The **Metro M4 Express** does not. The Thumb emitter costs about 19.5 KB and
  the board has 488 KB of firmware flash, so the build overflows. Dropping
  `ulab`, `synthio` and `audiofilters` makes it fit — and note that this costs
  the board `ulab`, which would otherwise be worth timing as a third variant.
- The **muzi Base Duo** already sets the flag in its `mpconfigboard.mk`.

## The benchmark harness

The `code.py` that drives the comparison — interleaved runs, warmup,
checksum verification, and the viper module it is measured against — is not in
this repository. It lives with the benchmark write-up, which also records the
measured numbers and explains, against the CircuitPython source, *why* viper
loses.

One trap worth repeating here, because it silently produces a meaningless
result: **do not put a `pixels.py` next to `pixels.mpy`.** A `.py` shadows a
`.mpy` of the same name, so the viper baseline quietly degrades to plain
bytecode and the speedup you measure is against the wrong thing.

## Licence

MIT. The kernel is derived from [turbo](https://github.com/mikeysklar/turbo),
also MIT.
