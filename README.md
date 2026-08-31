# Rust CircuitPython modules, by example

Native CircuitPython modules (`.mpy`) whose logic is written in Rust, with a
thin MicroPython C shim binding them to the interpreter.

| Module | Source | What it is |
| --- | --- | --- |
| `hello` | [rust-module-demo](rust-module-demo) | The smallest useful example: strings, integers, floats and a raised exception across the boundary |
| `adafruit_ticks` | [adafruit-bundle-modules-rust](adafruit-bundle-modules-rust/adafruit_ticks) | A Rust port of the Adafruit bundle's `adafruit_ticks` |
| `adafruit_debouncer` | [adafruit-bundle-modules-rust](adafruit-bundle-modules-rust/adafruit_debouncer) | A Rust port of the Adafruit bundle's `adafruit_debouncer` |

All of them target **ESP32-S3** (`ARCH = xtensawin`). A native `.mpy` is built
per architecture, so these will not load on another chip family, and the
firmware has to have been built with `CIRCUITPY_ENABLE_MPY_NATIVE=1`, which is
off by default.

Prebuilt modules are attached to each [release](../../releases).

## Building

Each module's own README has the detail. In short, three things have to be in
place that a normal Rust build does not need:

1. **The `esp` toolchain**, from [`espup`](https://github.com/esp-rs/espup).
   It supplies both the Xtensa Rust fork and the `xtensa-esp32s3-elf` GCC.
   `cargo +nightly` is not a substitute — see the module READMEs for why.
2. **A Python with `pyelftools` and `ar`**, because the linker is
   CircuitPython's `tools/mpy_ld.py`.
3. **A CircuitPython tree** at `MPY_DIR`, from the
   [`cpy-rust` branch of this fork](https://github.com/fede2cr/circuitpython/tree/cpy-rust).
   Stock upstream will not do; see below.

```bash
. ~/export-esp.sh
make -C rust-module-demo/natmod \
    MPY_DIR=/path/to/circuitpython \
    PYTHON=/path/to/venv/bin/python
```

`MPY_DIR` defaults to a `circuitpython` checkout sitting beside this repository,
so if you have one there you can leave it off.

## Why a fork of CircuitPython

The tree is used only as a build-time toolchain — nothing in CircuitPython refers
back here — but `tools/mpy_ld.py`, the native `.mpy` linker, has two defects that
are invisible when the input objects come from GAS and fatal when they come from
LLVM, which is what Rust uses.

**Literal-section keys collided.** `build_got_xtensa()` keyed literals by
`filename + offset`. With `-ffunction-sections` every function gets its own
`.literal.<name>` section, each starting at offset 0, so entries from different
sections overwrote each other and the wrong address was loaded at runtime. The
key now includes the section name.

**The `l32r` relocation was miscomputed, twice over.** The old line read

```python
l32r_imm16 = (l32r_imm16 + reloc >> 2) & 0xFFFF
```

In Python `+` binds tighter than `>>`, so this is `(imm16 + reloc) >> 2`, not
`imm16 + (reloc >> 2)`. That is only harmless when the incoming `imm16` is zero
— which is what GAS leaves, and what LLVM does not: it pre-encodes a guess such
as `0xFFFE`. The fix computes the offset from the final addresses instead of
adjusting the assembler's guess, and asserts the result falls in `l32r`'s legal
range of `[-0x40000, -4]` rather than silently wrapping.

Both are upstream MicroPython bugs rather than CircuitPython ones, and neither
has been submitted upstream yet. When they are, this can go back to a stock
checkout.

## CI

[`.github/workflows/build.yml`](.github/workflows/build.yml) builds and tests
every module on each push. Pushing a tag that starts with `v` runs the same
build and then attaches the `.mpy` files to a GitHub release.

```bash
git tag v0.1.0
git push origin v0.1.0
```
