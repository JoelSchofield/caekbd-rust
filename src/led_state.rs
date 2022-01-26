use smart_leds::RGB8;

pub enum LedMode {
    RAINBOW
}

// #[derive(Copy, Clone)]
// struct RGB {
//     r: u8,
//     g: u8,
//     b: u8
// }

// impl RGB {
//     pub fn new() -> Self {
//         Self {
//             r: 0,
//             g: 0,
//             b: 0
//         }
//     }

//     pub fn clear(&mut self) {
//         self.r = 0;
//         self.g = 0;
//         self.b = 0;
//     }
// }

// impl From<RGB8> for RGB {
//     fn from(item: RGB8) -> Self {
//         RGB { r: item.r, g: item.g, b: item.}
//     }
// }

pub struct LedState<const NUM_LEDS: usize> {
    leds: [RGB8; NUM_LEDS],
    wheel_positions: [u8; NUM_LEDS],
    tick_count: u32,
    led_mode: LedMode
}

impl<const NUM_LEDS: usize> LedState<NUM_LEDS> {
    pub fn new() -> Self {
        let mut ret = Self { 
            leds: [RGB8 {r: 0, g: 0, b: 0}; NUM_LEDS],
            wheel_positions: [0; NUM_LEDS],
            tick_count: 0,
            led_mode: LedMode::RAINBOW
        };

        ret.set_mode(LedMode::RAINBOW);
        return ret;
    }

    pub fn clear(&mut self) {
        for led in self.leds.iter_mut() {
            led.r = 0;
            led.g = 0;
            led.b = 0;
        }
    }

    pub fn set_mode(&mut self, mode: LedMode) {
        match mode {
            LedMode::RAINBOW => {
                self.init_rainbow();
            },
        }
    }

    fn init_rainbow(&mut self) {
        self.led_mode = LedMode::RAINBOW;

        let step = u8::MAX as f32 / NUM_LEDS as f32;
        
        for (i, wheel_pos) in self.wheel_positions.iter_mut().enumerate() {
            *wheel_pos = (i as f32 * step) as u8;
        } 
    }

    fn tick_rainbow(&mut self) {
        if self.tick_count < 10 {
            self.tick_count += 1;
            return;
        }
        else {
            self.tick_count = 0;
            for i in 0..NUM_LEDS {
                self.leds[i] = Self::wheel_rgb(self.wheel_positions[i]);
                self.wheel_positions[i] = self.wheel_positions[i].wrapping_add(1);
            }
        }
    }

    pub fn tick(&mut self) {
        // TODO: Add modes
        match self.led_mode {
            LedMode::RAINBOW => self.tick_rainbow()
        }
    }

    fn wheel_rgb(mut wheel_pos: u8) -> RGB8 {
        wheel_pos = 255 - wheel_pos;

        let mut rgb = RGB8 {r: 0, g: 0, b: 0};

        if wheel_pos < 85 {
            rgb.r = 255 - wheel_pos * 3;
            rgb.g = 0;
            rgb.b = wheel_pos * 3;
            
        }
        else if wheel_pos < 170 {
            wheel_pos -= 85;
            rgb.r = 0;
            rgb.g = wheel_pos * 3;
            rgb.b = 255 - wheel_pos * 3;
        }
        else {
            wheel_pos -= 170;
            rgb.r = wheel_pos * 3;
            rgb.g = 255 - wheel_pos * 3;
            rgb.b = 0;
        }
        
        return rgb;
    }

   pub fn get_grb(&self) -> [RGB8; NUM_LEDS] {
        let mut ret = self.leds.clone();

        for grb in ret.iter_mut() {
            let temp_r = grb.r;
            grb.r = grb.g;
            grb.g = temp_r;
        }

        return ret;
   }
}