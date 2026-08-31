# Rust-backed CircuitPython hello module prototype

This directory contains a minimal Rust prototype for a CircuitPython-compatible module.

## Goals

- demonstrate a Rust implementation for a simple CircuitPython module
- keep the public semantics easy to map to a future C ABI or MicroPython wrapper
- provide a first implementation target for ESP32-S3 experimentation

## Example API

- `hello_greet(name)` -> string
- `hello_add(a, b)` -> integer
- `hello_sub(a, b)` -> integer
- `hello_mul(a, b)` -> integer
- `hello_div(a, b)` -> float
- `hello_calc(op, a, b)` -> integer

## Build

### Which toolchain, and why it has to be that one

`Cargo.toml` opens with `cargo-features = ["panic-immediate-abort"]`, which is an
unstable *cargo* feature. Stable cargo refuses the manifest before it reads
anything else:

```
error: failed to parse manifest at `.../Cargo.toml`

Caused by:
  the cargo feature `panic-immediate-abort` requires a nightly version of Cargo,
  but this is the `stable` channel
```

The obvious fix is not the right one. **`cargo +nightly` will not work** unless
that nightly is recent enough to know the feature at all — an older one gets past
the channel check and then fails differently, which is more confusing than the
error it replaced:

```
Caused by:
  unknown cargo feature `panic-immediate-abort`
```

What this crate needs is the **`esp` toolchain**, installed by
[`espup`](https://github.com/esp-rs/espup). It is a nightly, it is the fork with
the Xtensa target this module is built for, and its cargo is new enough:

```bash
rustup toolchain list          # 'esp' should be listed
cargo +esp --version           # must be new enough to know the feature
```

Three ways to select it, in increasing order of how much typing they save:

```bash
# 1. per command
cargo +esp test --lib

# 2. for one shell
export RUSTUP_TOOLCHAIN=esp

# 3. for this directory, permanently -- create rust-toolchain.toml beside
#    Cargo.toml and every cargo command run from here picks it up
printf '[toolchain]\nchannel = "esp"\n' > rust-toolchain.toml
```

`natmod/Makefile` does not depend on any of this: it passes `+$(RUST_TOOLCHAIN)`
explicitly, defaulting to `esp`. Override it with `make RUST_TOOLCHAIN=<name>` if
yours is installed under a different name.

### Tests

```bash
cargo +esp test --lib
```

`--lib` is not optional. A plain `cargo test` also builds the doc-tests, whose
harness wants unwinding panics, and this crate is `no_std` with `panic = "abort"`:

```
error: unwinding panics are not supported without std
```

`cargo +esp test -Zpanic-abort-tests` works too, and does run the doc-tests.

### The static library

```bash
cargo +esp build --release \
    -Zbuild-std=core,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem \
    --target xtensa-esp32s3-none-elf
```

`-Zbuild-std` is not optional either. `panic = "immediate-abort"` in the release
profile applies only to this crate's own code; without rebuilding `core` under the
same strategy the two disagree:

```
error: the crate `core` was compiled with a panic strategy which is incompatible
with `immediate-abort`
```

Which is why the bare `cargo build --release` this file used to recommend never
produced a usable artifact. In practice you never run this by hand — `natmod/Makefile`
passes all of the above and then goes on to produce the `.mpy`.

### The .mpy

`natmod/` links the crate's static library against a thin MicroPython shim
(`hello.c`) into a native `.mpy` that CircuitPython can `import` at runtime.
It needs three things that are not part of a normal Rust build.

**1. The chip-specific Xtensa GCC.** `espup` installs it; `export-esp.sh` puts it
on `PATH`:

```bash
. ~/export-esp.sh
```

It must be the `xtensa-esp32s3-elf-` driver, not the generic `xtensa-esp-elf-`
one that sits in the same directory. With no chip dynconfig the generic driver
silently builds big-endian Xtensa, which links and then does not run. Skipping
this step gives:

```
make: xtensa-esp32s3-elf-gcc: No such file or directory
```

**2. A Python with `pyelftools` and `ar`.** The linker is
`circuitpython/tools/mpy_ld.py`, and the system Python has neither:

```bash
python3 -m venv .venv && .venv/bin/pip install pyelftools ar
```

Missing either one fails at a different step, several seconds apart, so it is
worth installing both up front:

```
ModuleNotFoundError: No module named 'elftools'
RuntimeError: Please run 'pip install ar' to link .a files
```

The second only bites modules that set `LINK_RUNTIME = 1` — this one does, to
resolve Rust's float intrinsics out of libgcc.

**3. A CircuitPython tree**, at `MPY_DIR` (default: a `circuitpython` checkout
beside this repository). It is used only as a build-time toolchain — nothing in
CircuitPython refers back here — but it has to be the
[`cpy-rust` branch of this fork](https://github.com/fede2cr/circuitpython/tree/cpy-rust)
rather than stock upstream, whose `.mpy` linker mangles Rust's output. The
[repository README](../README.md#why-a-fork-of-circuitpython) explains how.

Then:

```bash
. ~/export-esp.sh
cd rust-module-demo/natmod
make PYTHON=$(realpath -s ../../.venv/bin/python)
```

`realpath -s` because a relative `PYTHON` makes Python warn about `sys.prefix` on
every invocation, and plain `realpath` resolves the venv's `python` symlink back
to the system interpreter — which brings the `elftools` error straight back.

The build reports the section sizes and writes `hello.mpy` (about 1.4 kB):

```
arch:         EM_XTENSA
text size:    1137
rodata size:  120
bss size:     0
GOT entries:  23
GEN hello.mpy
```

`make clean` if a rebuild picks up a stale object — the shim is not rebuilt when
`dynruntime.h` changes.

### Running it

`ARCH = xtensawin`, so the result loads on an ESP32-S3 board and nothing else; a
`.mpy` is built per architecture. The firmware also has to have been compiled with
`CIRCUITPY_ENABLE_MPY_NATIVE=1`, which is off by default.

Copy `hello.mpy` to the drive (top level, or `lib/`) and the module's own name is
what you import:

```python
import hello

hello.greet("world")
hello.add(2, 3)
hello.calc("mul", 6, 7)
```

This project builds as a static library for upstream integration with CircuitPython and a native Rust library for local verification.
