#![allow(missing_docs)]

/*
Shamelessly stolen from keyberons matrix.rs, modified to add a small delay between
switching pins back to high.
TODO: Implement properly using some sort of delay trait?
    embedded_hal::blocking::delay::DelayUs
 */

use embedded_hal::digital::v2::{InputPin, OutputPin};
use keyberon::matrix::PressedKeys;

pub struct SlowMatrix<C, R, const CS: usize, const RS: usize>
where
    C: InputPin,
    R: OutputPin,
{
    cols: [C; CS],
    rows: [R; RS],
}

impl<C, R, const CS: usize, const RS: usize> SlowMatrix<C, R, CS, RS>
where
    C: InputPin,
    R: OutputPin,
{
    pub fn new<E>(cols: [C; CS], rows: [R; RS]) -> Result<Self, E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        let mut res = Self { cols, rows };
        res.clear()?;
        Ok(res)
    }
    pub fn clear<E>(&mut self) -> Result<(), E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        for r in self.rows.iter_mut() {
            r.set_high()?;
        }
        Ok(())
    }
    pub fn get<E>(&mut self) -> Result<PressedKeys<CS, RS>, E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        let mut keys = PressedKeys::default();

        for (ri, row) in (&mut self.rows).iter_mut().enumerate() {
            row.set_low()?;
            for (ci, col) in (&self.cols).iter().enumerate() {
                if col.is_low()? {
                    keys.0[ri][ci] = true;
                }
            }
            row.set_high()?;
            // Add delay into original matrix.rs impl, allowing each row enough
            // time to return to high before checking next row (avoiding false
            // keypresses).
            cortex_m::asm::delay(100);
        }
        Ok(keys)
    }
}
