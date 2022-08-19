use embedded_hal::spi::blocking::{SpiDevice, SpiBus, SpiBusWrite, SpiBusRead};
use register::Register;
use command::Command;

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

    pub fn reset(&mut self) -> Result<(), MyError<SPI::Error>> {
        self.execute_command(Command::SetDefault)?;

        Ok(())
    }

    pub fn read_register(&mut self, register: Register) -> Result<u8, MyError<SPI::Error>> {
        let mut buf = [0; 1];

        self.spi.transaction(|bus| {
            bus.write(&[register.read_address()])?;
            bus.read(&mut buf)
        }).map_err(MyError::Spi)?;

        Ok(buf[0])
    }

    pub fn write_register(&mut self, register: Register, val: u8) -> Result<(), MyError<SPI::Error>> {
        self.spi.transaction(|bus| {
            bus.write(&[register.write_address(), val])
        }).map_err(MyError::Spi)?;

        Ok(())
    }

    pub fn execute_command(&mut self, command: Command) -> Result<(), MyError<SPI::Error>> {
        self.spi.transaction(|bus| {
            bus.write(&[command.command_pattern()])
        }).map_err(MyError::Spi)?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MyError<SPI> {
    Spi(SPI),
    // Add other errors for your driver here.
}