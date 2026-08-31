# Builds the `adafruit_debouncer` native .mpy module: Rust logic + a thin MicroPython shim.
#
# The CircuitPython tree is only used as a build-time toolchain here; nothing in
# CircuitPython refers back to this project.

MPY_DIR ?= ../../../../circuitpython

MOD = adafruit_debouncer
SRC = adafruit_debouncer.c
ARCH = xtensawin

# dynruntime.mk hardcodes the original ESP32 prefix and resolves libgcc/libm at
# include time, so this has to win before then. The generic `xtensa-esp-elf-`
# driver must not be used: with no chip dynconfig it builds big-endian Xtensa.
override CROSS = xtensa-esp32s3-elf-

# The duration/interval properties are floats, so libgcc's soft-float helpers
# have to be linked in.
LINK_RUNTIME = 1

RUST_CRATE_DIR = ..
RUST_WORKSPACE_DIR = ../..
RUST_TARGET ?= xtensa-esp32s3-none-elf
RUST_TOOLCHAIN ?= esp
RUST_LIB = $(RUST_WORKSPACE_DIR)/target/$(RUST_TARGET)/release/libadafruit_debouncer.a
RUST_OBJ = $(BUILD)/adafruit_debouncer_rust.o

# Link only the crate's own object; libgcc/libm already supply the compiler
# intrinsics that Rust's compiler_builtins would otherwise duplicate. The
# adafruit_ticks helpers are #[inline], so no second object is needed.
SRC_O += $(RUST_OBJ)

include $(MPY_DIR)/py/dynruntime.mk

$(RUST_OBJ): $(RUST_LIB)
	$(ECHO) "AR $@"
	$(Q)cd $(BUILD) && $(CROSS)ar x $(abspath $(RUST_LIB)) \
		$$($(CROSS)ar t $(abspath $(RUST_LIB)) | grep '^adafruit_debouncer') \
		&& mv adafruit_debouncer*.o $(notdir $@)

$(RUST_LIB): $(RUST_CRATE_DIR)/src/lib.rs $(RUST_CRATE_DIR)/Cargo.toml $(RUST_WORKSPACE_DIR)/adafruit_ticks/src/lib.rs
	$(ECHO) "CARGO $(RUST_TARGET)"
	$(Q)cd $(RUST_CRATE_DIR) && cargo +$(RUST_TOOLCHAIN) build --release \
		-Zunstable-options \
		-Zbuild-std=core,compiler_builtins \
		-Zbuild-std-features=compiler-builtins-mem \
		--target $(RUST_TARGET)

.PHONY: rust-tests
# --lib skips the doctest pass, which would build the no_std lib with unwinding
# panics; the esp toolchain is needed for the unstable profile settings.
rust-tests:
	$(Q)cd $(RUST_CRATE_DIR) && cargo +$(RUST_TOOLCHAIN) test --lib
