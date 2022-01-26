#![no_std]
#![no_main]

use panic_halt as _;

mod hack;
mod ws2812_pio;

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {

    use cortex_m::prelude::_embedded_hal_watchdog_Watchdog;
    use cortex_m::prelude::_embedded_hal_watchdog_WatchdogEnable;
    use rp_pico::{
        hal::{self, clocks::init_clocks_and_plls, watchdog::Watchdog, Sio, Clock, pio::PIOExt},
        XOSC_CRYSTAL_FREQ,
        
    };
    use embedded_hal::digital::v2::{InputPin, OutputPin};
    use embedded_time::duration::units::*;
    use rp_pico::hal::gpio::DynPin;
    use rp_pico::hal::usb::UsbBus;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::Layout;
    use keyberon::matrix::{Matrix, PressedKeys};
    use usb_device::class_prelude::*;
    //use usb_device::device::UsbDeviceState;
    use smart_leds::{SmartLedsWrite, White, RGBW};
    use crate::hack::Ws2812;
    use embedded_hal::timer::CountDown;
    use crate::ws2812_pio::Ws2812Direct;

    const SCAN_TIME_US: u32 = 1000;
    const NUM_LEDS: usize = 17;

    static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<rp_pico::hal::usb::UsbBus>> = None;

    #[rustfmt::skip]
    pub static LAYERS: keyberon::layout::Layers = keyberon::layout::layout! {
        {[ // 0
            Q W E R T Y
        ]}
        {[ // 1
        1 2 3 4 5 6
        ]}
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
        matrix: Matrix<DynPin, DynPin, 6, 1>,
        layout: Layout,
        #[lock_free]
        debouncer: Debouncer<PressedKeys<6, 1>>,
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

        let gpio_col2 = pins.gpio2;
        let gpio_col3 = pins.gpio3;
        let gpio_col4 = pins.gpio6;
        let gpio_col5 = pins.gpio7;
        let gpio_col6 = pins.gpio8;
        let gpio_col7 = pins.gpio9;

        let gpio_row1 = pins.gpio20;

        let ws2812_timer = hal::timer::Timer::new(c.device.TIMER, &mut resets);

        let mut sleepy = ws2812_timer.count_down();
        sleepy.start(10.milliseconds());

        let (mut pio, sm0, _, _, _) = c.device.PIO0.split(&mut resets);

        let mut ws = Ws2812Direct::new(
            pins.gpio13.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
        );

        let mut wheel_val: u8 = 0;

        loop {
            use smart_leds::{SmartLedsWrite, RGB8};
            let colour: RGB8 = wheel_rgb(wheel_val).into();
            ws.write([colour; NUM_LEDS].iter().copied()).unwrap();

            wheel_val = wheel_val.wrapping_add(1);

            let _ = nb::block!(sleepy.wait());
        };

        let matrix: Matrix<DynPin, DynPin, 6, 1> = cortex_m::interrupt::free(move |_cs| {
            Matrix::new(
                [
                    gpio_col2.into_pull_up_input().into(),
                    gpio_col3.into_pull_up_input().into(),
                    gpio_col4.into_pull_up_input().into(),
                    gpio_col5.into_pull_up_input().into(),
                    gpio_col6.into_pull_up_input().into(),
                    gpio_col7.into_pull_up_input().into()
                ],
                [gpio_row1.into_push_pull_output().into()],
            )
        })
        .unwrap();

        let layout = Layout::new(LAYERS);
        let debouncer: keyberon::debounce::Debouncer<keyberon::matrix::PressedKeys<6, 1>> =
            Debouncer::new(PressedKeys::default(), PressedKeys::default(), 30);

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

//    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, layout])]
//    fn handle_event(mut c: handle_event::Context, event: Option<layout::Event>) {
//        let report: key_code::KbHidReport = c.shared.layout.lock(|l| l.keycodes().collect());
//        if !c
//            .shared
//            .usb_class
//            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
//        {
//            return;
//        }
//        if c.shared.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
//            return;
//        }
//        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
//    }

    #[task(
        binds = TIMER_IRQ_0,
        priority = 1,
        shared = [matrix, debouncer, watchdog, timer, alarm, layout, usb_class],
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
            c.shared.layout.lock(|l| l.event(event));
        }
        c.shared.layout.lock(|l| l.tick());

        let report: key_code::KbHidReport = c.shared.layout.lock(|l| l.keycodes().collect());

        if c.shared.usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
        }
    }

    fn wheel_rgb(mut wheel_pos: u8) -> (u8, u8, u8) {
        wheel_pos = 255 - wheel_pos;

        let r: u8;
        let g: u8;
        let b: u8;

        if wheel_pos < 85 {
            r = 255 - wheel_pos * 3;
            g = 0;
            b = wheel_pos * 3;
            
        }
        else if wheel_pos < 170 {
            wheel_pos -= 85;
            r = 0;
            g = wheel_pos * 3;
            b = 255 - wheel_pos * 3;
        }
        else {
            wheel_pos -= 170;
            r = wheel_pos * 3;
            g = 255 - wheel_pos * 3;
            b = 0;
        }
        
        return (r, g, b);
    }
}


/*
# The Physical Pins
#                       COL0       COL1       COL2       COL3       COL4       COL5       COL6       COL7       COL8        COL9        COL10       COL11       COL12       COL13       COL14       COL15
keyboard_cols = [ board.GP0, board.GP1, board.GP2, board.GP3, board.GP6, board.GP7, board.GP8, board.GP9, board.GP10, board.GP11, board.GP12, board.GP14, board.GP15, board.GP16, board.GP6, board.GP18 ]
#                       ROW0        ROW1        ROW2        ROW3        ROW4
keyboard_rows = [ board.GP19, board.GP20, board.GP21, board.GP22, board.GP26 ]

# The Pin Matrix
keyboard_cols_array = []
keyboard_rows_array = []

# Make all col pin objects inputs with pullups.
for pin in keyboard_cols:
    key_pin = digitalio.DigitalInOut(pin)           
    key_pin.direction = digitalio.Direction.OUTPUT
    key_pin.value = False
    keyboard_cols_array.append(key_pin)
    
# Make all row pin objects inputs with pullups
for pin in keyboard_rows:
    key_pin = digitalio.DigitalInOut(pin)
    key_pin.direction = digitalio.Direction.INPUT
    key_pin.pull = digitalio.Pull.DOWN
    keyboard_rows_array.append(key_pin)
*/