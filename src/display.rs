use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};
use tinybmp::Bmp;

const BONGO_IDLE: &[u8] = include_bytes!("../images/bongo_1.bmp");
const BONGO_TAP_1: &[u8] = include_bytes!("../images/bongo_2.bmp");
const BONGO_TAP_2: &[u8] = include_bytes!("../images/bongo_3.bmp");
const BONGO_TAP: [&[u8]; 2] = [include_bytes!("../images/bongo_2.bmp"),  include_bytes!("../images/bongo_3.bmp")];

pub struct CaeDisplay<I> {
    display: Ssd1306<I2CInterface<I>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
    bongo_cnt: usize,
    last_keypress: u32,
    tick_cnt: u32
}

impl<I> CaeDisplay<I>
where
    I: embedded_hal::blocking::i2c::Write,
{
    /// Create new builder with a default I2C address of 0x3C
    pub fn new(i2c: I) -> Self
    where
        I: embedded_hal::blocking::i2c::Write,
    {
        // Create the I²C display interface:
        let interface = ssd1306::I2CDisplayInterface::new(i2c);

        // Create a driver instance and initialize:
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        let mut display = Self { 
            display,
            bongo_cnt: 0,
            last_keypress: 0,
            tick_cnt: 0
        };
        
        display.draw_image(BONGO_IDLE);

        return display;
    }

    fn draw_image(&mut self, bytes: &[u8]) {
        self.display.clear();
        let bmp = Bmp::<BinaryColor>::from_slice(bytes).unwrap();
        Image::new(&bmp, Point::new(0, 0))
            .draw(&mut self.display)
            .unwrap();
        self.display.flush().unwrap();
    }

    pub fn handle_keypress(&mut self) {
        match self.bongo_cnt {
            0 => self.draw_image(BONGO_TAP_1),
            _ => self.draw_image(BONGO_TAP_2)
        };

        self.last_keypress = self.tick_cnt;

        // There must be some better way to do this with iterators, however I couldnt
        // find a way to store an iterator in the struct; the typing looks overcomplicated
        // for something that should be simple..
        self.bongo_cnt += 1;
        if self.bongo_cnt >= BONGO_TAP.len() {
            self.bongo_cnt = 0;
        }
    }

    // TODO: Fix this crap to use a proper SM and real timing
    pub fn tick(&mut self) {
        self.tick_cnt += 1;
        
        if (self.last_keypress + 400) == self.tick_cnt {
            self.draw_image(BONGO_IDLE);
        }
    }

    pub fn _test_draw_text(&mut self) {
        // Create a text style for drawing the font:
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X18_BOLD)
            .text_color(BinaryColor::On)
            .build();

        // Empty the display:
        self.display.clear();
        // Draw 3 lines of text:
        Text::with_baseline("Joelteon!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();

        self.display.flush().unwrap();
    }
}
