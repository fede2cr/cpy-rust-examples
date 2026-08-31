# Builds the `hello` native .mpy module: Rust logic + a thin MicroPython shim.
#
# The CircuitPython tree is only used as a build-time toolchain here; nothing in
# CircuitPython refers back to this project.

MPY_DIR ?= ../../../circuitpython

MOD = hello
SRC = hello.c
ARCH = xtensawin

# Pull in libgcc/libm so Rust's float intrinsics (e.g. __divsf3) resolve.
LINK_RUNTIME = 1

# dynruntime.mk hardcodes the original ESP32 prefix and resolves libgcc/libm at
# include time, so this has to win before then. The generic `xtensa-esp-elf-`
# driver must not be used: with no chip dynconfig it builds big-endian Xtensa.
override CROSS = xtensa-esp32s3-elf-

RUST_CRATE_DIR = ..
RUST_TARGET ?= xtensa-esp32s3-none-elf
RUST_TOOLCHAIN ?= esp
RUST_LIB = $(RUST_CRATE_DIR)/target/$(RUST_TARGET)/release/libcircuitpython_rust_hello.a
RUST_OBJ = $(BUILD)/rust_hello.o

# Link only the crate's own object; libgcc/libm already supply the compiler
# intrinsics that Rust's compiler_builtins would otherwise duplicate.
SRC_O += $(RUST_OBJ)

include $(MPY_DIR)/py/dynruntime.mk

$(RUST_OBJ): $(RUST_LIB)
	$(ECHO) "AR $@"
	$(Q)cd $(BUILD) && $(CROSS)ar x $(abspath $(RUST_LIB)) \
		$$($(CROSS)ar t $(abspath $(RUST_LIB)) | grep '^circuitpython_rust_hello') \
		&& mv circuitpython_rust_hello*.o $(notdir $@)

$(RUST_LIB): $(RUST_CRATE_DIR)/src/lib.rs $(RUST_CRATE_DIR)/Cargo.toml
	$(ECHO) "CARGO $(RUST_TARGET)"
	$(Q)cd $(RUST_CRATE_DIR) && cargo +$(RUST_TOOLCHAIN) build --release \
		-Zunstable-options \
		-Zbuild-std=core,compiler_builtins \
		-Zbuild-std-features=compiler-builtins-mem \
		--target $(RUST_TARGET)

.PHONY: rust-tests
rust-tests:
	$(Q)cd $(RUST_CRATE_DIR) && cargo test
