#![no_std]
#![no_main]

use panic_halt as _;

mod ws2812_pio;
mod slow_matrix;
mod led_state;

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {
    use cortex_m::prelude::_embedded_hal_watchdog_Watchdog;
    use cortex_m::prelude::_embedded_hal_watchdog_WatchdogEnable;
    use keyberon::layout::CustomEvent;
    use rp_pico::{
        XOSC_CRYSTAL_FREQ,
        hal::{
            self, clocks::init_clocks_and_plls, watchdog::Watchdog, Sio, Clock, rosc,
            pio::{PIOExt, SM0},
            gpio::pin::bank0::Gpio13
        }
    };
    use rp_pico::pac::PIO0;
    use embedded_time::duration::units::*;
    use rp_pico::hal::gpio::DynPin;
    use rp_pico::hal::usb::UsbBus;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::Layout;
    use keyberon::action::Action;
    use keyberon::matrix::PressedKeys;
    use usb_device::class_prelude::*;
    use smart_leds::SmartLedsWrite;
    use crate::ws2812_pio::Ws2812Direct;
    use crate::slow_matrix::SlowMatrix;
    use crate::led_state::{LedState, LedMode};

    const SCAN_TIME_US: u32 = 1000;
    const NUM_LEDS: usize = 17;
    const NUM_COLUMNS: usize = 16;
    const NUM_ROWS: usize = 5;
    
    static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<rp_pico::hal::usb::UsbBus>> = None;
    
    /*
    [ Keycode.ESCAPE, Keycode.ONE, Keycode.TWO, Keycode.THREE, Keycode.FOUR, Keycode.FIVE, Keycode.SIX, Keycode.SEVEN, Keycode.EIGHT, Keycode.NINE, Keycode.ZERO, Keycode.MINUS, Keycode.EQUALS, None, Keycode.BACKSPACE, Keycode.DELETE ],
    [ Keycode.TAB, None, Keycode.Q, Keycode.W, Keycode.E, Keycode.R, Keycode.T, Keycode.Y, Keycode.U, Keycode.I, Keycode.O, Keycode.P, Keycode.LEFT_BRACKET, Keycode.RIGHT_BRACKET, Keycode.BACKSLASH, Keycode.PRINT_SCREEN ],
    [ Keycode.CAPS_LOCK , None, Keycode.A, Keycode.S, Keycode.D, Keycode.F, Keycode.G, Keycode.H, Keycode.J, Keycode.K, Keycode.L, Keycode.SEMICOLON, Keycode.QUOTE, Keycode.ENTER, None, Keycode.UP_ARROW ],
    [ Keycode.LEFT_SHIFT, None, Keycode.Z, Keycode.X, Keycode.C, Keycode.V, Keycode.B, Keycode.N, Keycode.M, Keycode.COMMA, Keycode.PERIOD, Keycode.FORWARD_SLASH, None, Keycode.RIGHT_SHIFT, None, Keycode.DOWN_ARROW ],
    [ Keycode.LEFT_CONTROL, Keycode.LEFT_GUI, Keycode.LEFT_ALT, None, None, None, Keycode.SPACEBAR, None, None, None, Keycode.RIGHT_ALT, function_key_layer_hold(2), None, Keycode.APPLICATION, Keycode.RIGHT_CONTROL, function_key_layer_hold(1) ] ] 
    
    [ [ Keycode.GRAVE_ACCENT, Keycode.F1, Keycode.F2, Keycode.F3, Keycode.F4, Keycode.F5, Keycode.F6, Keycode.F7, Keycode.F8, Keycode.F9, Keycode.F10, Keycode.F11, Keycode.F12, None, None, Keycode.HOME ],
    [ macro1(), None, None, Keycode.UP_ARROW, None, None, None, None, None, None, None, None, None, None, None, Keycode.END ],
    [ Keycode.KEYPAD_NUMLOCK, None, Keycode.LEFT_ARROW, Keycode.DOWN_ARROW, Keycode.RIGHT_ARROW, None, None, None, None, None, None, None, None, Keycode.INSERT, None, Keycode.PAGE_UP],
    [ Keycode.LEFT_SHIFT, None, None, None, None, None, None, None, None, None, None, None, None, None, None, Keycode.PAGE_DOWN],
    [ Keycode.LEFT_CONTROL, Keycode.SCROLL_LOCK, None, None, None, None, Keycode.SPACEBAR, None, None, None, Keycode.RIGHT_ALT, None, None, Keycode.APPLICATION, Keycode.RIGHT_CONTROL, function_key_layer_hold(1) ] ],
    
    [ [ board_reload(), None, None, None, None, None, None, None, None, None, None, None, None, None, None, ConsumerControlCode.SCAN_PREVIOUS_TRACK],
    [ None, None, lighting_mode(-1), lighting_toggle_on_off(), lighting_mode(1), None, None, None, None, None, None, None, None, None, None, ConsumerControlCode.SCAN_NEXT_TRACK],
    [ None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, ConsumerControlCode.VOLUME_INCREMENT],
    [ None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, ConsumerControlCode.VOLUME_DECREMENT],
    [ None, None, None, None, None, None, ConsumerControlCode.PLAY_PAUSE, None, None, None, None, function_key_layer_hold(2), None, None, None, ConsumerControlCode.MUTE] ] ]
    */

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum CustomActions {
        SetModeRainbow,
        SetModeLightning,
        SetModeChase,
        RestartToUf2
    }

    const ACTION_SET_MODE_RAINBOW: Action<CustomActions> = Action::Custom(CustomActions::SetModeRainbow);
    const ACTION_SET_MODE_LIGHTNING: Action<CustomActions> = Action::Custom(CustomActions::SetModeLightning);
    const ACTION_SET_MODE_CHASE: Action<CustomActions> = Action::Custom(CustomActions::SetModeChase);
    const ACTION_RESTART_TO_UF2: Action<CustomActions> = Action::Custom(CustomActions::RestartToUf2);

    #[rustfmt::skip]
    pub static LAYERS: keyberon::layout::Layers<CustomActions> = keyberon::layout::layout! {
        {
            [Escape     1 2 3 4 5 6 7 8 9 0 - = n BSpace Delete  ]
            [Tab      n Q W E R T Y U I O P '[' ']' '\\' PScreen ]
            [CapsLock n A S D F G H J K L ; '\'' Enter n Up      ]
            [LShift   n Z X C V B N M , . / n   RShift n Down    ]
            [LCtrl LGui LAlt n n n Space n n n RAlt n n Application RCtrl (1) ]

        }
        {
            [t {ACTION_SET_MODE_RAINBOW} {ACTION_SET_MODE_LIGHTNING} {ACTION_SET_MODE_CHASE} t t t t t t t t t t t {ACTION_RESTART_TO_UF2} ]
            [t t t t t t t t t t t t t t t t ]
            [t t t t t t t t t t t t t t t t ]
            [t t t t t t t t t t t t t Up t t ]
            [t t t t t t t t t t t Left t Down Right (1) ]
        }
    };

    #[shared]
    struct Shared {
        usb_dev: usb_device::device::UsbDevice<'static, rp_pico::hal::usb::UsbBus>,
        usb_class: keyberon::hid::HidClass<
            'static,
            rp_pico::hal::usb::UsbBus,
            keyberon::keyboard::Keyboard<()>,
        >,
        timer: hal::timer::Timer,
        alarm: hal::timer::Alarm0,
        #[lock_free]
        watchdog: hal::watchdog::Watchdog,
        #[lock_free]
        matrix: SlowMatrix<DynPin, DynPin, NUM_COLUMNS, NUM_ROWS>,
        layout: Layout<CustomActions>,
        #[lock_free]
        debouncer: Debouncer<PressedKeys<NUM_COLUMNS, NUM_ROWS>>,
        #[lock_free]
        led_driver: Ws2812Direct<PIO0, SM0, Gpio13>,
        #[lock_free]
        led_state: LedState<rosc::RingOscillator<rosc::Enabled>, NUM_LEDS>,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut resets = c.device.RESETS;
        let mut watchdog = Watchdog::new(c.device.WATCHDOG);
        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(c.device.SIO);
        let pins = hal::gpio::Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let rng = rosc::RingOscillator::new(c.device.ROSC).initialize();

        /*
        # Physical Pins
        #                       COL0       COL1       COL2       COL3       COL4       COL5       COL6       COL7       COL8        COL9        COL10       COL11       COL12       COL13       COL14       COL15
        keyboard_cols = [ board.GP0, board.GP1, board.GP2, board.GP3, board.GP6, board.GP7, board.GP8, board.GP9, board.GP10, board.GP11, board.GP12, board.GP14, board.GP15, board.GP16, board.GP17, board.GP18 ]
        #                       ROW0        ROW1        ROW2        ROW3        ROW4
        keyboard_rows = [ board.GP19, board.GP20, board.GP21, board.GP22, board.GP26 ]
        */
        let gpio_col0 = pins.gpio0;
        let gpio_col1 = pins.gpio1;
        let gpio_col2 = pins.gpio2;
        let gpio_col3 = pins.gpio3;
        let gpio_col4 = pins.gpio6;
        let gpio_col5 = pins.gpio7;
        let gpio_col6 = pins.gpio8;
        let gpio_col7 = pins.gpio9;
        let gpio_col8 = pins.gpio10;
        let gpio_col9 = pins.gpio11;
        let gpio_col10 = pins.gpio12;
        let gpio_col11 = pins.gpio14;
        let gpio_col12 = pins.gpio15;
        let gpio_col13 = pins.gpio16;
        let gpio_col14 = pins.gpio17;
        let gpio_col15 = pins.gpio18;

        let gpio_row0 = pins.gpio19;
        let gpio_row1 = pins.gpio20;
        let gpio_row2 = pins.gpio21;
        let gpio_row3 = pins.gpio22;
        let gpio_row4 = pins.gpio26;
        
        // delay for power on
        for _ in 0..1000 {
            cortex_m::asm::nop();
        }

        let (mut pio, sm0, _, _, _) = c.device.PIO0.split(&mut resets);

        let led_driver = Ws2812Direct::new(
            pins.gpio13.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        let led_state: LedState<rosc::RingOscillator<rosc::Enabled>, NUM_LEDS> = LedState::new(rng);

        let matrix: SlowMatrix<DynPin, DynPin, NUM_COLUMNS, NUM_ROWS> = cortex_m::interrupt::free(move |_cs| {
            SlowMatrix::new(
                [
                    gpio_col0.into_pull_up_input().into(),
                    gpio_col1.into_pull_up_input().into(),
                    gpio_col2.into_pull_up_input().into(),
                    gpio_col3.into_pull_up_input().into(),
                    gpio_col4.into_pull_up_input().into(),
                    gpio_col5.into_pull_up_input().into(),
                    gpio_col6.into_pull_up_input().into(),
                    gpio_col7.into_pull_up_input().into(),
                    gpio_col8.into_pull_up_input().into(),
                    gpio_col9.into_pull_up_input().into(),
                    gpio_col10.into_pull_up_input().into(),
                    gpio_col11.into_pull_up_input().into(),
                    gpio_col12.into_pull_up_input().into(),
                    gpio_col13.into_pull_up_input().into(),
                    gpio_col14.into_pull_up_input().into(),
                    gpio_col15.into_pull_up_input().into(),
                ],
                [
                    gpio_row0.into_push_pull_output().into(),
                    gpio_row1.into_push_pull_output().into(),
                    gpio_row2.into_push_pull_output().into(),
                    gpio_row3.into_push_pull_output().into(),
                    gpio_row4.into_push_pull_output().into()
                ],
            )
        })
        .unwrap();

        let layout = Layout::new(LAYERS);
        let debouncer: keyberon::debounce::Debouncer<keyberon::matrix::PressedKeys<NUM_COLUMNS, NUM_ROWS>> =
            Debouncer::new(PressedKeys::default(), PressedKeys::default(), 10);

        let mut timer = hal::Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());
        alarm.enable_interrupt(&mut timer);

        let usb_bus = UsbBusAllocator::new(UsbBus::new(
            c.device.USBCTRL_REGS,
            c.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));
        unsafe {
            USB_BUS = Some(usb_bus);
        }
        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        // Start watchdog and feed it with the lowest priority task at 1000hz
        watchdog.start(10_000.microseconds());

        (
            Shared {
                usb_dev,
                usb_class,
                timer,
                alarm,
                watchdog,
                matrix,
                layout,
                debouncer,
                led_driver,
                led_state
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let mut usb_d = c.shared.usb_dev;
        let mut usb_c = c.shared.usb_class;
        usb_d.lock(|d| {
            usb_c.lock(|c| {
                if d.poll(&mut [c]) {
                    c.poll();
                }
            })
        });
    }

    #[task(
        binds = TIMER_IRQ_0,
        priority = 1,
        shared = [matrix, debouncer, watchdog, timer, alarm, layout, usb_class, led_driver, led_state],
    )]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let timer = c.shared.timer;
        let alarm = c.shared.alarm;
        (timer, alarm).lock(|t, a| {
            a.clear_interrupt(t);
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });

        c.shared.watchdog.feed();
        for event in c
            .shared
            .debouncer
            .events(c.shared.matrix.get().unwrap())
        {
            if event.is_press() {
                c.shared.led_state.handle_keypress();
            }
            c.shared.layout.lock(|l| l.event(event));
        }

        let mut mode = None;

        c.shared.layout.lock(|l| {
            let custom_action = l.tick();

            match custom_action {
                CustomEvent::Press(CustomActions::SetModeRainbow) => mode = Some(LedMode::Rainbow),
                CustomEvent::Press(CustomActions::SetModeLightning) => mode = Some(LedMode::Lightning),
                CustomEvent::Press(CustomActions::SetModeChase) => mode = Some(LedMode::Chase),
                CustomEvent::Press(CustomActions::RestartToUf2) => hal::rom_data::reset_to_usb_boot(0, 0),
                _ => (),
            }
        });

        match mode {
            Some(x) => c.shared.led_state.set_mode(x),
            None => ()
        }

        let report: key_code::KbHidReport = c.shared.layout.lock(|l| l.keycodes().collect());

        if c.shared.usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
        }
        
        //led_state.handle_keypress(random_num, random_index);
        c.shared.led_state.tick();
        let data = c.shared.led_state.get_grb();
        c.shared.led_driver.write(data.iter().copied()).unwrap();
    }
}
