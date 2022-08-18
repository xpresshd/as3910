#![cfg_attr(not(feature = "std"), no_std)]

use embedded_hal::spi::blocking::{SpiDevice, SpiBus, SpiBusWrite, SpiBusRead};

mod register;
mod command;

pub struct AS3910<SPI> {
    spi: SPI,
}

impl<SPI> AS3910<SPI>
where
    SPI: SpiDevice,
    SPI::Bus: SpiBus,
{
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    pub fn read_foo(&mut self) -> Result<[u8; 2], MyError<SPI::Error>> {
        let mut buf = [0; 2];

        // `transaction` asserts and deasserts CS for us. No need to do it manually!
        self.spi.transaction(|bus| {
            bus.write(&[0x90])?;
            bus.read(&mut buf)
        }).map_err(MyError::Spi)?;

        Ok(buf)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MyError<SPI> {
    Spi(SPI),
    // Add other errors for your driver here.
}