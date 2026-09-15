// Native CircuitPython module `pixels_rs`, implemented in Rust (see ../src/lib.rs).
//
// `mandel_row` is signature-compatible with turbo's viper function of the same
// name, so the two can be timed side by side in one session, or this module
// renamed to `pixels` to drop in over it.

#include "py/dynruntime.h"

extern void rust_mandel_row(uint8_t *out, int32_t width, int32_t dx, int32_t cy, int32_t max_iter);
extern uint32_t rust_mandel_sum(int32_t width, int32_t height, int32_t max_iter);

// At opt-level 3 LLVM turns the max_iter <= 0 path of mandel_row into a bulk
// zero-fill. A natmod links no libc, so route it to the runtime's own memset.
#if defined(__arm__)
void __aeabi_memclr(void *dest, size_t n) {
    mp_fun_table.memset_(dest, 0, n);
}
#else
void *memset(void *s, int c, size_t n) {
    return mp_fun_table.memset_(s, c, n);
}
#endif

static mp_obj_t mod_mandel_row(size_t n_args, const mp_obj_t *args) {
    mp_buffer_info_t buf;
    mp_get_buffer_raise(args[0], &buf, MP_BUFFER_WRITE);

    mp_int_t width = mp_obj_get_int(args[1]);
    // viper's ptr8 would write past the end here; the cost of checking is one
    // comparison per row, not per pixel.
    if (width < 0 || (size_t)width > buf.len) {
        mp_raise_ValueError(MP_ERROR_TEXT("width exceeds buffer"));
    }

    rust_mandel_row(buf.buf, width, mp_obj_get_int(args[2]), mp_obj_get_int(args[3]),
        mp_obj_get_int(args[4]));
    return mp_const_none;
}
static MP_DEFINE_CONST_FUN_OBJ_VAR_BETWEEN(mod_mandel_row_obj, 5, 5, mod_mandel_row);

static mp_obj_t mod_mandel_sum(mp_obj_t width_in, mp_obj_t height_in, mp_obj_t max_iter_in) {
    uint32_t total = rust_mandel_sum(mp_obj_get_int(width_in), mp_obj_get_int(height_in),
        mp_obj_get_int(max_iter_in));
    return mp_obj_new_int_from_uint(total);
}
static MP_DEFINE_CONST_FUN_OBJ_3(mod_mandel_sum_obj, mod_mandel_sum);

static mp_obj_t mod_turbo_bench(void) {
    return mp_obj_new_int_from_uint(rust_mandel_sum(160, 120, 64));
}
static MP_DEFINE_CONST_FUN_OBJ_0(mod_turbo_bench_obj, mod_turbo_bench);

mp_obj_t mpy_init(mp_obj_fun_bc_t *self, size_t n_args, size_t n_kw, mp_obj_t *args) {
    MP_DYNRUNTIME_INIT_ENTRY

    mp_store_global(MP_QSTR_mandel_row, MP_OBJ_FROM_PTR(&mod_mandel_row_obj));
    mp_store_global(MP_QSTR_mandel_sum, MP_OBJ_FROM_PTR(&mod_mandel_sum_obj));
    mp_store_global(MP_QSTR__turbo_bench, MP_OBJ_FROM_PTR(&mod_turbo_bench_obj));

    MP_DYNRUNTIME_INIT_EXIT
}
