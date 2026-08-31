# Adafruit bundle modules, in Rust

Rust ports of two modules from the
[Adafruit CircuitPython bundle](https://github.com/adafruit/Adafruit_CircuitPython_Bundle),
built as native `.mpy` modules rather than bytecode.

| Module | Upstream |
| --- | --- |
| [`adafruit_ticks`](adafruit_ticks) | [Adafruit_CircuitPython_Ticks](https://github.com/adafruit/Adafruit_CircuitPython_Ticks) |
| [`adafruit_debouncer`](adafruit_debouncer) | [Adafruit_CircuitPython_Debouncer](https://github.com/adafruit/Adafruit_CircuitPython_Debouncer) |

This is a cargo workspace, so the two crates share `Cargo.lock` and a `target/`.
Each has its own `natmod/` producing one `.mpy`.

## Building

The prerequisites and the reasoning behind them are written up once, in
[`../rust-module-demo/README.md`](../rust-module-demo/README.md); nothing here
differs from them. In short:

```bash
. ~/export-esp.sh
cargo +esp test --lib                       # from either crate directory
make -C adafruit_ticks/natmod \
    MPY_DIR=/path/to/circuitpython \
    PYTHON=/path/to/venv/bin/python
```

`adafruit_ticks` happens to build without the `ar` package because it does not
set `LINK_RUNTIME`; `adafruit_debouncer` does set it, and does need it. Install
`pyelftools` and `ar` together and the distinction stops mattering.
