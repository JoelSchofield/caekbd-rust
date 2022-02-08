use embedded_graphics::{
    image::Image,
    mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};
use tinybmp::Bmp;

const BONGO_1: &[u8] = include_bytes!("../images/bongo_1.bmp");
const BONGO_2: &[u8] = include_bytes!("../images/bongo_2.bmp");
const BONGO_3: &[u8] = include_bytes!("../images/bongo_3.bmp");

pub struct CaeDisplay<I> {
    display: Ssd1306<I2CInterface<I>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
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

        Self { display }
    }

    pub fn test_draw(&mut self) {
        self.display.clear();
        let bmp = Bmp::<BinaryColor>::from_slice(BONGO_1).unwrap();
        Image::new(&bmp, Point::new(0, 0))
            .draw(&mut self.display)
            .unwrap();
        self.display.flush().unwrap();
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
