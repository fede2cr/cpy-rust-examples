# Builds the `pixels_rs` native .mpy module: Rust logic + a thin MicroPython shim.
#
# Two targets, selected with ARCH:
#   make ARCH=xtensawin   ESP32-S3, needs the `esp` Rust toolchain from espup
#   make ARCH=armv7emsp   SAMD51 / Cortex-M4F, needs nightly + rust-src
#
# The CircuitPython tree is only used as a build-time toolchain here; nothing in
# CircuitPython refers back to this project.

MPY_DIR ?= ../../../circuitpython
ARCH ?= xtensawin

MOD = pixels_rs
SRC = pixels_rs.c
# Keep each architecture's objects apart so both can be built from one checkout.
BUILD = build/$(ARCH)
DIST ?= ../../dist/$(ARCH)

ifeq ($(ARCH),xtensawin)
# dynruntime.mk hardcodes the original ESP32 prefix and resolves libgcc/libm at
# include time, so this has to win before then. The generic `xtensa-esp-elf-`
# driver must not be used: with no chip dynconfig it builds big-endian Xtensa.
override CROSS = xtensa-esp32s3-elf-
RUST_TARGET ?= xtensa-esp32s3-none-elf
RUST_TOOLCHAIN ?= esp
else ifeq ($(ARCH),armv7emsp)
# dynruntime.mk already selects arm-none-eabi- and the cortex-m4 hard-float ABI.
RUST_TARGET ?= thumbv7em-none-eabihf
RUST_TOOLCHAIN ?= nightly
else
$(error unsupported ARCH '$(ARCH)')
endif

RUST_CRATE_DIR = ..
RUST_LIB = $(RUST_CRATE_DIR)/target/$(RUST_TARGET)/release/libpixels_rs.a
RUST_OBJ = $(BUILD)/pixels_rs_rust.o

# Link only the crate's own object; libgcc/libm already supply the compiler
# intrinsics that Rust's compiler_builtins would otherwise duplicate.
SRC_O += $(RUST_OBJ)

include $(MPY_DIR)/py/dynruntime.mk

$(RUST_OBJ): $(RUST_LIB)
	$(ECHO) "AR $@"
	$(Q)cd $(BUILD) && $(CROSS)ar x $(abspath $(RUST_LIB)) \
		$$($(CROSS)ar t $(abspath $(RUST_LIB)) | grep '^pixels_rs') \
		&& mv pixels_rs*.o $(notdir $@)

$(RUST_LIB): $(RUST_CRATE_DIR)/src/lib.rs $(RUST_CRATE_DIR)/Cargo.toml
	$(ECHO) "CARGO $(RUST_TARGET)"
	$(Q)cd $(RUST_CRATE_DIR) && cargo +$(RUST_TOOLCHAIN) build --release \
		-Zunstable-options \
		-Zbuild-std=core,compiler_builtins \
		-Zbuild-std-features=compiler-builtins-mem \
		--target $(RUST_TARGET)

.PHONY: dist
dist: $(MOD).mpy
	$(Q)$(MKDIR) -p $(DIST)
	$(Q)cp $(MOD).mpy $(DIST)/$(MOD).mpy
	$(ECHO) "DIST $(DIST)/$(MOD).mpy"

.PHONY: rust-tests
# --lib skips the doctest pass, which would build the no_std lib with unwinding
# panics; the esp toolchain is needed for the unstable profile settings.
rust-tests:
	$(Q)cd $(RUST_CRATE_DIR) && cargo +esp test --lib --features std
