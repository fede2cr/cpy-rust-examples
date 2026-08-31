// Native CircuitPython module `hello`, implemented in Rust (see ../src/lib.rs).
//
// This shim only converts between MicroPython objects and the Rust C ABI; it is
// built into a native .mpy and is never compiled into CircuitPython itself.

#include "py/dynruntime.h"

#define HELLO_ERR_DIV_ZERO (-2)

// Upper bound for the greeting produced by the Rust side.
#define HELLO_GREET_BUF_SIZE 96

#define hello_type_ZeroDivisionError \
    (*(mp_obj_type_t *)(mp_load_global(MP_QSTR_ZeroDivisionError)))

extern int32_t rust_hello_add(int32_t a, int32_t b);
extern int32_t rust_hello_sub(int32_t a, int32_t b);
extern int32_t rust_hello_mul(int32_t a, int32_t b);
extern int32_t rust_hello_div(int32_t a, int32_t b, float *out);
extern int32_t rust_hello_calc(const byte *op, size_t op_len, int32_t a, int32_t b, int32_t *out);
extern int32_t rust_hello_greet(const byte *name, size_t name_len, byte *out, size_t out_cap);

static mp_obj_t hello_greet(mp_obj_t name_in) {
    size_t name_len;
    const char *name = mp_obj_str_get_data(name_in, &name_len);
    byte buf[HELLO_GREET_BUF_SIZE];
    int32_t written = rust_hello_greet((const byte *)name, name_len, buf, sizeof(buf));
    if (written < 0) {
        mp_raise_ValueError(MP_ERROR_TEXT("name too long"));
    }
    return mp_obj_new_str((const char *)buf, (size_t)written);
}
static MP_DEFINE_CONST_FUN_OBJ_1(hello_greet_obj, hello_greet);

static mp_obj_t hello_add(mp_obj_t a_in, mp_obj_t b_in) {
    return mp_obj_new_int(rust_hello_add(mp_obj_get_int(a_in), mp_obj_get_int(b_in)));
}
static MP_DEFINE_CONST_FUN_OBJ_2(hello_add_obj, hello_add);

static mp_obj_t hello_sub(mp_obj_t a_in, mp_obj_t b_in) {
    return mp_obj_new_int(rust_hello_sub(mp_obj_get_int(a_in), mp_obj_get_int(b_in)));
}
static MP_DEFINE_CONST_FUN_OBJ_2(hello_sub_obj, hello_sub);

static mp_obj_t hello_mul(mp_obj_t a_in, mp_obj_t b_in) {
    return mp_obj_new_int(rust_hello_mul(mp_obj_get_int(a_in), mp_obj_get_int(b_in)));
}
static MP_DEFINE_CONST_FUN_OBJ_2(hello_mul_obj, hello_mul);

static mp_obj_t hello_div(mp_obj_t a_in, mp_obj_t b_in) {
    float result = 0.0f;
    int32_t rc = rust_hello_div(mp_obj_get_int(a_in), mp_obj_get_int(b_in), &result);
    if (rc == HELLO_ERR_DIV_ZERO) {
        mp_raise_msg(&hello_type_ZeroDivisionError, MP_ERROR_TEXT("division by zero"));
    }
    return mp_obj_new_float(result);
}
static MP_DEFINE_CONST_FUN_OBJ_2(hello_div_obj, hello_div);

static mp_obj_t hello_calc(mp_obj_t op_in, mp_obj_t a_in, mp_obj_t b_in) {
    size_t op_len;
    const char *op = mp_obj_str_get_data(op_in, &op_len);
    int32_t result = 0;
    int32_t rc = rust_hello_calc((const byte *)op, op_len,
        mp_obj_get_int(a_in), mp_obj_get_int(b_in), &result);
    if (rc != 0) {
        mp_raise_ValueError(MP_ERROR_TEXT("unsupported operation"));
    }
    return mp_obj_new_int(result);
}
static MP_DEFINE_CONST_FUN_OBJ_3(hello_calc_obj, hello_calc);

mp_obj_t mpy_init(mp_obj_fun_bc_t *self, size_t n_args, size_t n_kw, mp_obj_t *args) {
    MP_DYNRUNTIME_INIT_ENTRY

    mp_store_global(MP_QSTR_greet, MP_OBJ_FROM_PTR(&hello_greet_obj));
    mp_store_global(MP_QSTR_add, MP_OBJ_FROM_PTR(&hello_add_obj));
    mp_store_global(MP_QSTR_sub, MP_OBJ_FROM_PTR(&hello_sub_obj));
    mp_store_global(MP_QSTR_mul, MP_OBJ_FROM_PTR(&hello_mul_obj));
    mp_store_global(MP_QSTR_div, MP_OBJ_FROM_PTR(&hello_div_obj));
    mp_store_global(MP_QSTR_calc, MP_OBJ_FROM_PTR(&hello_calc_obj));

    MP_DYNRUNTIME_INIT_EXIT
}
