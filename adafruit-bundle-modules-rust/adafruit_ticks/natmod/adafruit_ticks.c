// Native CircuitPython module `adafruit_ticks`, implemented in Rust (see ../src/lib.rs).
//
// This shim only converts between MicroPython objects and the Rust C ABI; it is
// built into a native .mpy and is never compiled into CircuitPython itself.

#include "py/dynruntime.h"

#define TICKS_ERR_OVERFLOW (-1)

#define ticks_type_OverflowError \
    (*(mp_obj_type_t *)(mp_load_global(MP_QSTR_OverflowError)))

extern int32_t rust_ticks_add(int32_t ticks, int32_t delta, int32_t *out);
extern int32_t rust_ticks_diff(int32_t ticks1, int32_t ticks2);
extern int32_t rust_ticks_less(int32_t ticks1, int32_t ticks2);

static mp_obj_t mod_ticks_add(mp_obj_t ticks_in, mp_obj_t delta_in) {
    int32_t result = 0;
    int32_t rc = rust_ticks_add(mp_obj_get_int(ticks_in), mp_obj_get_int(delta_in), &result);
    if (rc == TICKS_ERR_OVERFLOW) {
        mp_raise_msg(&ticks_type_OverflowError, MP_ERROR_TEXT("ticks interval overflow"));
    }
    return mp_obj_new_int(result);
}
static MP_DEFINE_CONST_FUN_OBJ_2(mod_ticks_add_obj, mod_ticks_add);

static mp_obj_t mod_ticks_diff(mp_obj_t ticks1_in, mp_obj_t ticks2_in) {
    return mp_obj_new_int(rust_ticks_diff(mp_obj_get_int(ticks1_in), mp_obj_get_int(ticks2_in)));
}
static MP_DEFINE_CONST_FUN_OBJ_2(mod_ticks_diff_obj, mod_ticks_diff);

static mp_obj_t mod_ticks_less(mp_obj_t ticks1_in, mp_obj_t ticks2_in) {
    return mp_obj_new_bool(rust_ticks_less(mp_obj_get_int(ticks1_in), mp_obj_get_int(ticks2_in)));
}
static MP_DEFINE_CONST_FUN_OBJ_2(mod_ticks_less_obj, mod_ticks_less);

mp_obj_t mpy_init(mp_obj_fun_bc_t *self, size_t n_args, size_t n_kw, mp_obj_t *args) {
    MP_DYNRUNTIME_INIT_ENTRY

    // ticks_ms only reads the hardware clock, so re-export it as-is; the Python
    // library's monotonic fallbacks are for ports that lack supervisor.
    mp_obj_t supervisor = mp_import_name(MP_QSTR_supervisor, mp_const_none, MP_OBJ_NEW_SMALL_INT(0));
    mp_store_global(MP_QSTR_ticks_ms, mp_load_attr(supervisor, MP_QSTR_ticks_ms));

    mp_store_global(MP_QSTR_ticks_add, MP_OBJ_FROM_PTR(&mod_ticks_add_obj));
    mp_store_global(MP_QSTR_ticks_diff, MP_OBJ_FROM_PTR(&mod_ticks_diff_obj));
    mp_store_global(MP_QSTR_ticks_less, MP_OBJ_FROM_PTR(&mod_ticks_less_obj));

    MP_DYNRUNTIME_INIT_EXIT
}
