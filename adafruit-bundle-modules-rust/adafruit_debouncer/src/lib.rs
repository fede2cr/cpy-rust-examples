//! Rust implementation behind the CircuitPython `adafruit_debouncer` native module.
//!
//! Ports the state machines of
//! <https://github.com/adafruit/Adafruit_CircuitPython_Debouncer>.
//! Time is supplied by the caller rather than read here, so the machines stay
//! pure and can be driven from host tests; the shim passes `supervisor.ticks_ms`.
//!
//! The exported `extern "C"` surface is `no_std` and allocation-free so the
//! compiled object can be linked into a MicroPython native `.mpy` module.

#![cfg_attr(not(test), no_std)]

use adafruit_ticks::ticks_diff;

pub const DEBOUNCED_STATE: u8 = 0x01;
pub const UNSTABLE_STATE: u8 = 0x02;
pub const CHANGED_STATE: u8 = 0x04;

/// Mirrored by `debouncer_state_t` in the C shim; the layout is the ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DebouncerState {
    pub last_bounce_ticks: i32,
    pub last_duration_ticks: i32,
    pub state_changed_ticks: i32,
    pub interval_ticks: i32,
    pub state: u8,
}

impl DebouncerState {
    pub fn new(interval_ticks: i32, initial: bool) -> Self {
        Self {
            last_bounce_ticks: 0,
            last_duration_ticks: 0,
            state_changed_ticks: 0,
            interval_ticks,
            state: if initial {
                DEBOUNCED_STATE | UNSTABLE_STATE
            } else {
                0
            },
        }
    }

    fn get(&self, bits: u8) -> bool {
        self.state & bits != 0
    }

    pub fn update(&mut self, current: bool, now_ticks: i32) {
        self.state &= !CHANGED_STATE;
        if current != self.get(UNSTABLE_STATE) {
            self.last_bounce_ticks = now_ticks;
            self.state ^= UNSTABLE_STATE;
        } else if ticks_diff(now_ticks, self.last_bounce_ticks) >= self.interval_ticks
            && current != self.get(DEBOUNCED_STATE)
        {
            self.last_bounce_ticks = now_ticks;
            self.state ^= DEBOUNCED_STATE;
            self.state |= CHANGED_STATE;
            self.last_duration_ticks = ticks_diff(now_ticks, self.state_changed_ticks);
            self.state_changed_ticks = now_ticks;
        }
    }

    pub fn value(&self) -> bool {
        self.get(DEBOUNCED_STATE)
    }

    pub fn rose(&self) -> bool {
        self.get(DEBOUNCED_STATE) && self.get(CHANGED_STATE)
    }

    pub fn fell(&self) -> bool {
        !self.get(DEBOUNCED_STATE) && self.get(CHANGED_STATE)
    }
}

/// Mirrored by `button_state_t` in the C shim; the layout is the ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonState {
    pub debouncer: DebouncerState,
    pub last_change_ms: i32,
    pub short_duration_ms: i32,
    pub long_duration_ms: i32,
    pub short_counter: i32,
    pub short_to_show: i32,
    pub value_when_pressed: u8,
    pub long_registered: u8,
    pub long_to_show: u8,
}

impl ButtonState {
    pub fn new(
        interval_ticks: i32,
        initial: bool,
        short_duration_ms: i32,
        long_duration_ms: i32,
        value_when_pressed: bool,
        now_ticks: i32,
    ) -> Self {
        Self {
            debouncer: DebouncerState::new(interval_ticks, initial),
            last_change_ms: now_ticks,
            short_duration_ms,
            long_duration_ms,
            short_counter: 0,
            short_to_show: 0,
            value_when_pressed: value_when_pressed as u8,
            long_registered: 0,
            long_to_show: 0,
        }
    }

    fn value_when_pressed(&self) -> bool {
        self.value_when_pressed != 0
    }

    pub fn pressed(&self) -> bool {
        if self.value_when_pressed() {
            self.debouncer.rose()
        } else {
            self.debouncer.fell()
        }
    }

    pub fn released(&self) -> bool {
        if self.value_when_pressed() {
            self.debouncer.fell()
        } else {
            self.debouncer.rose()
        }
    }

    pub fn update(&mut self, current: bool, now_ticks: i32) {
        self.debouncer.update(current, now_ticks);
        if self.pressed() {
            self.last_change_ms = now_ticks;
            self.short_counter += 1;
        } else if self.released() {
            self.last_change_ms = now_ticks;
            self.long_registered = 0;
        } else {
            let duration = ticks_diff(now_ticks, self.last_change_ms);
            if self.long_registered == 0
                && self.debouncer.value() == self.value_when_pressed()
                && duration > self.long_duration_ms
            {
                self.long_registered = 1;
                self.long_to_show = 1;
                self.short_to_show = self.short_counter - 1;
                self.short_counter = 0;
            } else if self.short_counter > 0
                && self.debouncer.value() != self.value_when_pressed()
                && duration > self.short_duration_ms
            {
                self.short_to_show = self.short_counter;
                self.short_counter = 0;
            } else {
                self.long_to_show = 0;
                self.short_to_show = 0;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_debouncer_size() -> usize {
    core::mem::size_of::<DebouncerState>()
}

#[no_mangle]
pub extern "C" fn rust_button_size() -> usize {
    core::mem::size_of::<ButtonState>()
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_init(
    state: *mut DebouncerState,
    interval_ticks: i32,
    initial: i32,
) {
    *state = DebouncerState::new(interval_ticks, initial != 0);
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_update(
    state: *mut DebouncerState,
    current: i32,
    now_ticks: i32,
) {
    (*state).update(current != 0, now_ticks);
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_value(state: *const DebouncerState) -> i32 {
    (*state).value() as i32
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_rose(state: *const DebouncerState) -> i32 {
    (*state).rose() as i32
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_fell(state: *const DebouncerState) -> i32 {
    (*state).fell() as i32
}

#[no_mangle]
pub unsafe extern "C" fn rust_debouncer_current_duration_ticks(
    state: *const DebouncerState,
    now_ticks: i32,
) -> i32 {
    ticks_diff(now_ticks, (*state).state_changed_ticks)
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn rust_button_init(
    state: *mut ButtonState,
    interval_ticks: i32,
    initial: i32,
    short_duration_ms: i32,
    long_duration_ms: i32,
    value_when_pressed: i32,
    now_ticks: i32,
) {
    *state = ButtonState::new(
        interval_ticks,
        initial != 0,
        short_duration_ms,
        long_duration_ms,
        value_when_pressed != 0,
        now_ticks,
    );
}

#[no_mangle]
pub unsafe extern "C" fn rust_button_update(state: *mut ButtonState, current: i32, now_ticks: i32) {
    (*state).update(current != 0, now_ticks);
}

#[no_mangle]
pub unsafe extern "C" fn rust_button_pressed(state: *const ButtonState) -> i32 {
    (*state).pressed() as i32
}

#[no_mangle]
pub unsafe extern "C" fn rust_button_released(state: *const ButtonState) -> i32 {
    (*state).released() as i32
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: i32 = 10;

    /// Drives the machine with one sample per millisecond.
    struct Clock {
        now: i32,
    }

    impl Clock {
        fn new() -> Self {
            Self { now: 0 }
        }

        fn feed(&mut self, d: &mut DebouncerState, level: bool, ms: i32) {
            for _ in 0..ms {
                self.now += 1;
                d.update(level, self.now);
            }
        }
    }

    #[test]
    fn initial_value_follows_first_reading() {
        assert!(!DebouncerState::new(INTERVAL, false).value());
        assert!(DebouncerState::new(INTERVAL, true).value());
    }

    #[test]
    fn bounces_shorter_than_interval_are_rejected() {
        let mut d = DebouncerState::new(INTERVAL, false);
        let mut c = Clock::new();
        // Chatter that never stays put for a full interval.
        for _ in 0..20 {
            c.feed(&mut d, true, INTERVAL - 5);
            c.feed(&mut d, false, INTERVAL - 5);
        }
        assert!(!d.value());
        assert!(!d.rose());
    }

    #[test]
    fn stable_change_is_accepted_and_reported_once() {
        let mut d = DebouncerState::new(INTERVAL, false);
        let mut c = Clock::new();
        c.feed(&mut d, true, INTERVAL + 2);
        assert!(d.value());
        // rose() is only true on the update that changed it.
        c.feed(&mut d, true, 1);
        assert!(!d.rose());
        assert!(d.value());

        c.feed(&mut d, false, INTERVAL + 2);
        assert!(!d.value());
    }

    #[test]
    fn last_duration_measures_the_previous_stable_period() {
        let mut d = DebouncerState::new(INTERVAL, false);
        let mut c = Clock::new();
        c.feed(&mut d, true, INTERVAL + 1);
        let first_change = d.state_changed_ticks;
        c.feed(&mut d, true, 50);
        c.feed(&mut d, false, INTERVAL + 1);
        assert_eq!(d.last_duration_ticks, d.state_changed_ticks - first_change);
    }

    /// Active-low button: the predicate reads false while pressed.
    fn button() -> ButtonState {
        ButtonState::new(INTERVAL, true, 200, 500, false, 0)
    }

    /// `short_count`, `long_press`, `pressed` and `released` are all true for
    /// exactly one update, so a polling loop has to sample them every time.
    #[derive(Default)]
    struct Events {
        shorts: Vec<i32>,
        longs: i32,
        presses: i32,
        releases: i32,
    }

    fn drive(b: &mut ButtonState, c: &mut Clock, level: bool, ms: i32, ev: &mut Events) {
        for _ in 0..ms {
            c.now += 1;
            b.update(level, c.now);
            if b.short_to_show > 0 {
                ev.shorts.push(b.short_to_show);
            }
            if b.long_to_show != 0 {
                ev.longs += 1;
            }
            if b.pressed() {
                ev.presses += 1;
            }
            if b.released() {
                ev.releases += 1;
            }
        }
    }

    #[test]
    fn single_short_press_counts_once() {
        let mut b = button();
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, true, 50, &mut ev);
        drive(&mut b, &mut c, false, 50, &mut ev); // press
        drive(&mut b, &mut c, true, 250, &mut ev); // release, then wait out short_duration
        assert_eq!(ev.shorts, vec![1]);
        assert_eq!(ev.longs, 0);
        assert_eq!(ev.presses, 1);
        assert_eq!(ev.releases, 1);
    }

    #[test]
    fn double_click_counts_twice() {
        let mut b = button();
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, true, 50, &mut ev);
        for _ in 0..2 {
            drive(&mut b, &mut c, false, 30, &mut ev);
            drive(&mut b, &mut c, true, 30, &mut ev);
        }
        drive(&mut b, &mut c, true, 250, &mut ev);
        assert_eq!(ev.shorts, vec![2]);
        assert_eq!(ev.presses, 2);
    }

    #[test]
    fn long_press_is_reported_and_suppresses_its_own_click() {
        let mut b = button();
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, true, 50, &mut ev);
        drive(&mut b, &mut c, false, 600, &mut ev); // held past long_duration
        assert_eq!(ev.longs, 1);
        assert!(ev.shorts.is_empty());
        assert_eq!(b.short_counter, 0);
    }

    #[test]
    fn click_then_long_press_reports_both() {
        let mut b = button();
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, true, 50, &mut ev);
        drive(&mut b, &mut c, false, 30, &mut ev); // click
        drive(&mut b, &mut c, true, 30, &mut ev);
        drive(&mut b, &mut c, false, 600, &mut ev); // hold
        assert_eq!(ev.longs, 1);
        assert_eq!(ev.shorts, vec![1]);
    }

    #[test]
    fn bouncy_press_is_still_a_single_click() {
        let mut b = button();
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, true, 50, &mut ev);
        // Contact chatter on the way down, then a clean hold.
        for _ in 0..5 {
            drive(&mut b, &mut c, false, 3, &mut ev);
            drive(&mut b, &mut c, true, 3, &mut ev);
        }
        drive(&mut b, &mut c, false, 40, &mut ev);
        drive(&mut b, &mut c, true, 250, &mut ev);
        assert_eq!(ev.presses, 1);
        assert_eq!(ev.shorts, vec![1]);
    }

    #[test]
    fn pressed_and_released_respect_value_when_pressed() {
        // Active-high button: pressed when the predicate reads true.
        let mut b = ButtonState::new(INTERVAL, false, 200, 500, true, 0);
        let mut c = Clock::new();
        let mut ev = Events::default();
        drive(&mut b, &mut c, false, 50, &mut ev);
        drive(&mut b, &mut c, true, 50, &mut ev);
        assert_eq!(ev.presses, 1);
        assert_eq!(ev.releases, 0);
        drive(&mut b, &mut c, false, 50, &mut ev);
        assert_eq!(ev.releases, 1);
    }

    #[test]
    fn state_survives_the_tick_counter_wrapping() {
        let mut d = DebouncerState::new(INTERVAL, false);
        // Start just below the 2**29 wrap point.
        let mut now = (1 << 29) - 5;
        for _ in 0..(INTERVAL + 5) {
            now = adafruit_ticks::ticks_add(now, 1).unwrap();
            d.update(true, now);
        }
        assert!(d.value());
    }

    #[test]
    fn c_abi_matches_the_rust_api() {
        let mut d = DebouncerState::new(INTERVAL, false);
        unsafe {
            rust_debouncer_init(&mut d, INTERVAL, 0);
            for now in 1..=(INTERVAL + 2) {
                rust_debouncer_update(&mut d, 1, now);
            }
            assert_eq!(rust_debouncer_value(&d), 1);
        }
        assert_eq!(rust_debouncer_size(), core::mem::size_of::<DebouncerState>());
        assert_eq!(rust_button_size(), core::mem::size_of::<ButtonState>());
    }
}
