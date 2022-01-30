use smart_leds::RGB8;
use rand_core::RngCore;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LedMode {
    Rainbow,
    Lightning,
    Chase,
    Chase2
}

pub struct LedState<R: RngCore, const NUM_LEDS: usize>
{
    leds: [RGB8; NUM_LEDS],
    wheel_positions: [u8; NUM_LEDS],
    tick_count: u32,
    led_mode: LedMode,
    chase_count: usize,
    rng: R
}

impl<R: RngCore, const NUM_LEDS: usize> LedState<R, NUM_LEDS> {
    pub fn new(rng: R) -> Self {
        let mut ret = Self { 
            leds: [RGB8 {r: 0, g: 0, b: 0}; NUM_LEDS],
            wheel_positions: [0; NUM_LEDS],
            tick_count: 0,
            led_mode: LedMode::Rainbow,
            chase_count: 0,
            rng
        };

        ret.set_mode(LedMode::Chase2);
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
            LedMode::Rainbow => {
                self.init_rainbow();
            },
            LedMode::Lightning => {
                self.init_lightning();
            },
            LedMode::Chase => {
                self.init_chase();
            },
            LedMode::Chase2 => {
                self.init_chase_2();
            }
        }
    }

    fn init_rainbow(&mut self) {
        self.led_mode = LedMode::Rainbow;

        let step = u8::MAX as f32 / NUM_LEDS as f32;
        
        for (i, wheel_pos) in self.wheel_positions.iter_mut().enumerate() {
            *wheel_pos = (i as f32 * step) as u8;
        } 
    }

    fn init_lightning(&mut self) {
        self.led_mode = LedMode::Lightning;
        self.clear();
    }

    fn init_chase(&mut self) {
        self.led_mode = LedMode::Chase;
        self.clear();
    }

    fn init_chase_2(&mut self) {
        self.led_mode = LedMode::Chase2;
        self.clear();
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

    fn tick_lightning(&mut self) {
        if self.tick_count < 10 {
            self.tick_count += 1;
            return;
        }
        else {
            self.tick_count = 0;

            if self.one_in_chance(20) {
                let random_key_index = self.rand_index(NUM_LEDS);
                
                if self.leds[random_key_index].r < 100 {
                    
                    if self.one_in_chance(2) {
                        self.leds[random_key_index].r = 200;
                        self.leds[random_key_index].g = 200;
                        self.leds[random_key_index].b = 0;
                    }
                    else {
                        self.leds[random_key_index].r = 200;
                        self.leds[random_key_index].g = 200;
                        self.leds[random_key_index].b = 70;
                    }
                }
            }

            for led in self.leds.iter_mut() {
                led.r = led.r.saturating_sub(2);
                led.g = led.g.saturating_sub(2);
                led.b = led.b.saturating_sub(2);
            }
        }
    }

    fn tick_chase(&mut self) {
        if self.tick_count < 10 {
            self.tick_count += 1;
            return;
        }
        else {
            self.tick_count = 0;
            for led in self.leds.iter_mut() {
                led.r = led.r.saturating_sub(1);
                led.g = led.g.saturating_sub(1);
                led.b = led.b.saturating_sub(1);
            }
        }
    }

    fn tick_chase_2(&mut self) {
        if self.tick_count < 100 {
            self.tick_count += 1;
            return;
        }
        else {
            self.tick_count = 0;

            self.chase_count += 1;
            if self.chase_count >= NUM_LEDS {
                self.chase_count = 0;
            }

            let chase_2 = (self.chase_count + NUM_LEDS / 2) % NUM_LEDS;

            self.wheel_positions[0] = self.wheel_positions[0].wrapping_add(10);
            self.leds[self.chase_count] = Self::wheel_rgb(self.wheel_positions[0]);
            self.leds[chase_2] = Self::wheel_rgb(self.wheel_positions[0]);
            
            for led in self.leds.iter_mut() {
                led.r = led.r.saturating_sub(20);
                led.g = led.g.saturating_sub(20);
                led.b = led.b.saturating_sub(20);
            }
        }
    }

    fn handle_keypress_lightning(&mut self) {
        let random_key_index = self.rand_index(NUM_LEDS);

        if self.one_in_chance(2) {
            self.leds[random_key_index].r = 255;
            self.leds[random_key_index].g = 255;
            self.leds[random_key_index].b = 0;
        }
        else {
            self.leds[random_key_index].r = 255;
            self.leds[random_key_index].g = 255;
            self.leds[random_key_index].b = 125;
        }
    }

    fn handle_keypress_chase(&mut self) {
        self.chase_count += 1;
        if self.chase_count >= NUM_LEDS {
            self.chase_count = 0;
        }

        self.wheel_positions[0] = self.wheel_positions[0].wrapping_add(10);
        self.leds[self.chase_count] = Self::wheel_rgb(self.wheel_positions[0])
    }

    pub fn handle_keypress(&mut self) {
        match self.led_mode {

            LedMode::Lightning => {
                self.handle_keypress_lightning();
            },
            LedMode::Chase => {
                self.handle_keypress_chase();
            },
            _ => ()
        }
    }

    fn one_in_chance(&mut self, chance: u32) -> bool {
        if self.rand_index(chance as usize) == 0 {
            true
        }
        else {
            false
        }
    }

    fn rand_index(&mut self, len: usize) -> usize {
        let random_num = self.rng.next_u32();
        let ret = random_num as usize % len;
        return ret;
    }

    pub fn tick(&mut self) {
        // TODO: Add modes
        match self.led_mode {
            LedMode::Rainbow => self.tick_rainbow(),
            LedMode::Lightning => self.tick_lightning(),
            LedMode::Chase => self.tick_chase(),
            LedMode::Chase2 => self.tick_chase_2(),
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