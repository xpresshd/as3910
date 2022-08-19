use embedded_hal_2_7 as hal;
use hal::blocking::spi;
use hal::digital::v2::OutputPin;

use crate::register::Register;

pub struct AS3910<SPI, CS> {
    spi: SPI,
    cs: CS,
}

impl<E, CS, SPI> AS3910<SPI, CS>
where
    SPI: spi::Transfer<u8, Error = E> + spi::Write<u8, Error = E>,
    CS: OutputPin,
{
    pub fn new(spi: SPI, cs: CS) -> Self {
        Self { spi, cs }
    }

    pub fn read(&mut self, reg: Register) -> Result<u8, E> {
        let mut buffer = [reg.read_address(), 0];

        self.with_cs_high(|mfr| {
            let buffer = mfr.spi.transfer(&mut buffer)?;

            Ok(buffer[1])
        })
    }

    fn read_many<'b>(&mut self, reg: Register, buffer: &'b mut [u8]) -> Result<&'b [u8], E> {
        let byte = reg.read_address();

        self.with_cs_high(move |mfr| {
            mfr.spi.transfer(&mut [byte])?;

            let n = buffer.len();
            for slot in &mut buffer[..n - 1] {
                *slot = mfr.spi.transfer(&mut [byte])?[0];
            }

            buffer[n - 1] = mfr.spi.transfer(&mut [0])?[0];

            Ok(&*buffer)
        })
    }

    pub fn write(&mut self, reg: Register, val: u8) -> Result<(), E> {
        self.with_cs_high(|mfr| mfr.spi.write(&[reg.write_address(), val]))
    }

    fn write_many(&mut self, reg: Register, bytes: &[u8]) -> Result<(), E> {
        self.with_cs_high(|mfr| {
            mfr.spi.write(&[reg.write_address()])?;
            mfr.spi.write(bytes)?;

            Ok(())
        })
    }

    fn with_cs_high<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.cs.set_high();
        let result = f(self);
        self.cs.set_low();

        result
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MyError<SPI> {
    Spi(SPI),
    // Add other errors for your driver here.
}
