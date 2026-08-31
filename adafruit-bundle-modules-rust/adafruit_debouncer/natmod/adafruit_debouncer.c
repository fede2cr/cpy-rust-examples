// Native CircuitPython module `adafruit_debouncer`, implemented in Rust (see ../src/lib.rs).
//
// This shim owns the Python-visible types and reads the clock and the input;
// every state transition happens in Rust. It is built into a native .mpy and is
// never compiled into CircuitPython itself.

#include "py/dynruntime.h"

#define TICKS_PER_SEC (1000)

// Mirrors DebouncerState in ../src/lib.rs. The layout is the ABI between the two
// halves, so mpy_init checks it against rust_debouncer_size().
typedef struct {
    int32_t last_bounce_ticks;
    int32_t last_duration_ticks;
    int32_t state_changed_ticks;
    int32_t interval_ticks;
    uint8_t state;
} debouncer_state_t;

// Mirrors ButtonState in ../src/lib.rs.
typedef struct {
    debouncer_state_t debouncer;
    int32_t last_change_ms;
    int32_t short_duration_ms;
    int32_t long_duration_ms;
    int32_t short_counter;
    int32_t short_to_show;
    uint8_t value_when_pressed;
    uint8_t long_registered;
    uint8_t long_to_show;
} button_state_t;

extern size_t rust_debouncer_size(void);
extern size_t rust_button_size(void);
extern void rust_debouncer_init(debouncer_state_t *state, int32_t interval_ticks, int32_t initial);
extern void rust_debouncer_update(debouncer_state_t *state, int32_t current, int32_t now_ticks);
extern int32_t rust_debouncer_value(const debouncer_state_t *state);
extern int32_t rust_debouncer_rose(const debouncer_state_t *state);
extern int32_t rust_debouncer_fell(const debouncer_state_t *state);
extern int32_t rust_debouncer_current_duration_ticks(const debouncer_state_t *state, int32_t now_ticks);
extern void rust_button_init(button_state_t *state, int32_t interval_ticks, int32_t initial,
    int32_t short_duration_ms, int32_t long_duration_ms, int32_t value_when_pressed, int32_t now_ticks);
extern void rust_button_update(button_state_t *state, int32_t current, int32_t now_ticks);
extern int32_t rust_button_pressed(const button_state_t *state);
extern int32_t rust_button_released(const button_state_t *state);

// The natmod's own .bss is copied into IRAM, which the GC does not scan, so
// every object reference has to live in the GC-allocated instance.
typedef struct {
    mp_obj_base_t base;
    mp_obj_t source;
    mp_obj_t ticks_ms;
    bool source_is_io;
    debouncer_state_t state;
} debouncer_obj_t;

typedef struct {
    mp_obj_base_t base;
    mp_obj_t source;
    mp_obj_t ticks_ms;
    bool source_is_io;
    button_state_t state;
} button_obj_t;

mp_obj_full_type_t mp_type_Debouncer;
mp_obj_full_type_t mp_type_Button;

static mp_obj_t import_ticks_ms(void) {
    mp_obj_t supervisor = mp_import_name(MP_QSTR_supervisor, mp_const_none, MP_OBJ_NEW_SMALL_INT(0));
    return mp_load_attr(supervisor, MP_QSTR_ticks_ms);
}

static int32_t now_ticks(mp_obj_t ticks_ms) {
    return mp_obj_get_int(mp_call_function_n_kw(ticks_ms, 0, 0, NULL));
}

// A DigitalInOut is used via its `value`; anything else is called.
static bool source_is_io(mp_obj_t source) {
    mp_obj_t dest[2];
    mp_load_method_maybe(source, MP_QSTR_value, dest);
    return dest[0] != MP_OBJ_NULL;
}

static int32_t read_source(mp_obj_t source, bool is_io) {
    mp_obj_t value = is_io
        ? mp_load_attr(source, MP_QSTR_value)
        : mp_call_function_n_kw(source, 0, 0, NULL);
    return mp_obj_is_true(value) ? 1 : 0;
}

static int32_t ticks_from_seconds(mp_obj_t seconds) {
    return (int32_t)(mp_obj_get_float(seconds) * TICKS_PER_SEC);
}

static mp_obj_t seconds_from_ticks(int32_t ticks) {
    return mp_obj_new_float((mp_float_t)ticks / TICKS_PER_SEC);
}

// Returns the value for `name`, or MP_OBJ_NULL if it is not a keyword this
// constructor accepts.
static mp_obj_t kwarg_get(size_t n_args, size_t n_kw, const mp_obj_t *args, qstr name) {
    for (size_t i = 0; i < n_kw; i++) {
        if (args[n_args + 2 * i] == MP_OBJ_NEW_QSTR(name)) {
            return args[n_args + 2 * i + 1];
        }
    }
    return MP_OBJ_NULL;
}

static mp_obj_t debouncer_make_new(const mp_obj_type_t *type, size_t n_args, size_t n_kw, const mp_obj_t *args) {
    mp_arg_check_num(n_args, n_kw, 1, 2, true);
    debouncer_obj_t *self = mp_obj_malloc(debouncer_obj_t, type);
    self->source = args[0];
    self->source_is_io = source_is_io(self->source);
    self->ticks_ms = import_ticks_ms();

    mp_obj_t interval = n_args >= 2 ? args[1] : kwarg_get(n_args, n_kw, args, MP_QSTR_interval);
    int32_t interval_ticks = interval == MP_OBJ_NULL ? 10 : ticks_from_seconds(interval);

    rust_debouncer_init(&self->state, interval_ticks, read_source(self->source, self->source_is_io));
    return MP_OBJ_FROM_PTR(self);
}

static mp_obj_t debouncer_update(size_t n_args, const mp_obj_t *args) {
    debouncer_obj_t *self = MP_OBJ_TO_PTR(args[0]);
    int32_t current = (n_args >= 2 && args[1] != mp_const_none)
        ? (mp_obj_is_true(args[1]) ? 1 : 0)
        : read_source(self->source, self->source_is_io);
    rust_debouncer_update(&self->state, current, now_ticks(self->ticks_ms));
    return mp_const_none;
}
static MP_DEFINE_CONST_FUN_OBJ_VAR_BETWEEN(debouncer_update_obj, 1, 2, debouncer_update);

// Handles the members shared by Debouncer and Button. Returns false if `attr` is
// not one of them, leaving dest untouched.
static bool debouncer_common_attr(mp_obj_t self_in, debouncer_state_t *state, mp_obj_t ticks_ms,
    qstr attr, mp_obj_t *dest) {
    if (dest[0] == MP_OBJ_SENTINEL) {
        if (attr == MP_QSTR_interval) {
            state->interval_ticks = ticks_from_seconds(dest[1]);
            dest[0] = MP_OBJ_NULL;
            return true;
        }
        return false;
    }
    if (attr == MP_QSTR_update) {
        dest[0] = MP_OBJ_FROM_PTR(&debouncer_update_obj);
        dest[1] = self_in;
    } else if (attr == MP_QSTR_value) {
        dest[0] = mp_obj_new_bool(rust_debouncer_value(state));
    } else if (attr == MP_QSTR_rose) {
        dest[0] = mp_obj_new_bool(rust_debouncer_rose(state));
    } else if (attr == MP_QSTR_fell) {
        dest[0] = mp_obj_new_bool(rust_debouncer_fell(state));
    } else if (attr == MP_QSTR_interval) {
        dest[0] = seconds_from_ticks(state->interval_ticks);
    } else if (attr == MP_QSTR_last_duration) {
        dest[0] = seconds_from_ticks(state->last_duration_ticks);
    } else if (attr == MP_QSTR_current_duration) {
        dest[0] = seconds_from_ticks(
            rust_debouncer_current_duration_ticks(state, now_ticks(ticks_ms)));
    } else {
        return false;
    }
    return true;
}

static void debouncer_attr(mp_obj_t self_in, qstr attr, mp_obj_t *dest) {
    debouncer_obj_t *self = MP_OBJ_TO_PTR(self_in);
    debouncer_common_attr(self_in, &self->state, self->ticks_ms, attr, dest);
}

static mp_obj_t button_make_new(const mp_obj_type_t *type, size_t n_args, size_t n_kw, const mp_obj_t *args) {
    mp_arg_check_num(n_args, n_kw, 1, 4, true);
    button_obj_t *self = mp_obj_malloc(button_obj_t, type);
    self->source = args[0];
    self->source_is_io = source_is_io(self->source);
    self->ticks_ms = import_ticks_ms();

    mp_obj_t short_ms = n_args >= 2 ? args[1] : kwarg_get(n_args, n_kw, args, MP_QSTR_short_duration_ms);
    mp_obj_t long_ms = n_args >= 3 ? args[2] : kwarg_get(n_args, n_kw, args, MP_QSTR_long_duration_ms);
    mp_obj_t when_pressed = n_args >= 4 ? args[3] : kwarg_get(n_args, n_kw, args, MP_QSTR_value_when_pressed);
    mp_obj_t interval = kwarg_get(n_args, n_kw, args, MP_QSTR_interval);

    rust_button_init(&self->state,
        interval == MP_OBJ_NULL ? 10 : ticks_from_seconds(interval),
        read_source(self->source, self->source_is_io),
        short_ms == MP_OBJ_NULL ? 200 : mp_obj_get_int(short_ms),
        long_ms == MP_OBJ_NULL ? 500 : mp_obj_get_int(long_ms),
        when_pressed != MP_OBJ_NULL && mp_obj_is_true(when_pressed),
        now_ticks(self->ticks_ms));
    return MP_OBJ_FROM_PTR(self);
}

static mp_obj_t button_update(size_t n_args, const mp_obj_t *args) {
    button_obj_t *self = MP_OBJ_TO_PTR(args[0]);
    int32_t current = (n_args >= 2 && args[1] != mp_const_none)
        ? (mp_obj_is_true(args[1]) ? 1 : 0)
        : read_source(self->source, self->source_is_io);
    rust_button_update(&self->state, current, now_ticks(self->ticks_ms));
    return mp_const_none;
}
static MP_DEFINE_CONST_FUN_OBJ_VAR_BETWEEN(button_update_obj, 1, 2, button_update);

static void button_attr(mp_obj_t self_in, qstr attr, mp_obj_t *dest) {
    button_obj_t *self = MP_OBJ_TO_PTR(self_in);
    if (dest[0] == MP_OBJ_SENTINEL) {
        if (attr == MP_QSTR_short_duration_ms) {
            self->state.short_duration_ms = mp_obj_get_int(dest[1]);
            dest[0] = MP_OBJ_NULL;
        } else if (attr == MP_QSTR_long_duration_ms) {
            self->state.long_duration_ms = mp_obj_get_int(dest[1]);
            dest[0] = MP_OBJ_NULL;
        } else if (attr == MP_QSTR_value_when_pressed) {
            self->state.value_when_pressed = mp_obj_is_true(dest[1]);
            dest[0] = MP_OBJ_NULL;
        } else {
            debouncer_common_attr(self_in, &self->state.debouncer, self->ticks_ms, attr, dest);
        }
        return;
    }
    if (attr == MP_QSTR_update) {
        dest[0] = MP_OBJ_FROM_PTR(&button_update_obj);
        dest[1] = self_in;
    } else if (attr == MP_QSTR_pressed) {
        dest[0] = mp_obj_new_bool(rust_button_pressed(&self->state));
    } else if (attr == MP_QSTR_released) {
        dest[0] = mp_obj_new_bool(rust_button_released(&self->state));
    } else if (attr == MP_QSTR_short_count) {
        dest[0] = mp_obj_new_int(self->state.short_to_show);
    } else if (attr == MP_QSTR_long_press) {
        dest[0] = mp_obj_new_bool(self->state.long_to_show);
    } else if (attr == MP_QSTR_short_duration_ms) {
        dest[0] = mp_obj_new_int(self->state.short_duration_ms);
    } else if (attr == MP_QSTR_long_duration_ms) {
        dest[0] = mp_obj_new_int(self->state.long_duration_ms);
    } else if (attr == MP_QSTR_value_when_pressed) {
        dest[0] = mp_obj_new_bool(self->state.value_when_pressed);
    } else if (attr == MP_QSTR_last_change_ms) {
        dest[0] = mp_obj_new_int(self->state.last_change_ms);
    } else {
        debouncer_common_attr(self_in, &self->state.debouncer, self->ticks_ms, attr, dest);
    }
}

mp_obj_t mpy_init(mp_obj_fun_bc_t *self, size_t n_args, size_t n_kw, mp_obj_t *args) {
    MP_DYNRUNTIME_INIT_ENTRY

    // The C structs above are hand-mirrored from Rust, so fail loudly on drift.
    if (rust_debouncer_size() != sizeof(debouncer_state_t)
        || rust_button_size() != sizeof(button_state_t)) {
        mp_raise_msg(&mp_type_RuntimeError, MP_ERROR_TEXT("debouncer ABI mismatch"));
    }

    mp_type_Debouncer.base.type = (void *)&mp_type_type;
    mp_type_Debouncer.flags = MP_TYPE_FLAG_NONE;
    mp_type_Debouncer.name = MP_QSTR_Debouncer;
    MP_OBJ_TYPE_SET_SLOT(&mp_type_Debouncer, make_new, debouncer_make_new, 0);
    MP_OBJ_TYPE_SET_SLOT(&mp_type_Debouncer, attr, debouncer_attr, 1);
    mp_store_global(MP_QSTR_Debouncer, MP_OBJ_FROM_PTR(&mp_type_Debouncer));

    mp_type_Button.base.type = (void *)&mp_type_type;
    mp_type_Button.flags = MP_TYPE_FLAG_NONE;
    mp_type_Button.name = MP_QSTR_Button;
    MP_OBJ_TYPE_SET_SLOT(&mp_type_Button, make_new, button_make_new, 0);
    MP_OBJ_TYPE_SET_SLOT(&mp_type_Button, attr, button_attr, 1);
    MP_OBJ_TYPE_SET_SLOT(&mp_type_Button, parent, &mp_type_Debouncer, 2);
    mp_store_global(MP_QSTR_Button, MP_OBJ_FROM_PTR(&mp_type_Button));

    MP_DYNRUNTIME_INIT_EXIT
}
