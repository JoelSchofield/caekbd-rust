use rp_pico::hal::timer::Timer;
use embedded_time::clock::Clock;

pub struct PicoClock<'a> {
    timer: &'a Timer
}

impl<'a> PicoClock<'a>
where 
    Self: Clock,
{
    pub fn new(timer: &'a Timer) -> Self {
        Self {
            timer
        }
    }
}

impl<'a> Clock for PicoClock<'a> {
    type T = u64;

    // Ticks occur every 1us. Define the scaling factor as 1Mhz
    const SCALING_FACTOR: embedded_time::rate::Fraction =
        embedded_time::rate::Fraction::new(1, 1000000);

    fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
        Ok(embedded_time::Instant::new(
            self.timer.get_counter()
        ))
    }
}
