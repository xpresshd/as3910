#![cfg_attr(not(feature = "std"), no_std)]

use command::Command;
use embedded_hal as hal;
use hal::blocking::delay;
use hal::blocking::spi;
use hal::digital::v2::InputPin;
use hal::digital::v2::OutputPin;

pub mod command;
mod picc;
pub mod register;

use crate::register::Register;

/// Answer To reQuest A
pub struct AtqA {
    bytes: [u8; 2],
}

pub enum Uid {
    /// Single sized UID, 4 bytes long
    Single(GenericUid<4>),
    /// Double sized UID, 7 bytes long
    Double(GenericUid<7>),
    /// Trip sized UID, 10 bytes long
    Triple(GenericUid<10>),
}

impl Uid {
    pub fn as_bytes(&self) -> &[u8] {
        match &self {
            Uid::Single(u) => u.as_bytes(),
            Uid::Double(u) => u.as_bytes(),
            Uid::Triple(u) => u.as_bytes(),
        }
    }
}

pub struct GenericUid<const T: usize>
where
    [u8; T]: Sized,
{
    /// The UID can have 4, 7 or 10 bytes.
    bytes: [u8; T],
    /// The SAK (Select acknowledge) byte returned from the PICC after successful selection.
    sak: picc::Sak,
}

impl<const T: usize> GenericUid<T> {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn is_compliant(&self) -> bool {
        self.sak.is_compliant()
    }
}

pub struct AS3910<SPI, CS, INTR, DELAY> {
    spi: SPI,
    cs: CS,
    /// Interrupt pin
    intr: INTR,
    delay: DELAY,
}

impl<E, OPE, CS, INTR, SPI, DELAY> AS3910<SPI, CS, INTR, DELAY>
where
    SPI: spi::Transfer<u8, Error = E> + spi::Write<u8, Error = E>,
    CS: OutputPin<Error = OPE>,
    INTR: InputPin<Error = OPE>,
    DELAY: delay::DelayMs<u16>,
{
    pub fn new(spi: SPI, cs: CS, intr: INTR, delay: DELAY) -> Result<Self, Error<E, OPE>> {
        let mut as3910 = Self {
            spi,
            cs,
            intr,
            delay,
        };
        as3910.reset()?;
        // TODO: investigate and write comment
        as3910.write_register(Register::RegulatedVoltageDefinition, 0xA8)?;

        as3910.execute_command(Command::CalibrateAntenna)?;

        as3910.delay.delay_ms(1);
        let val = as3910.read_register(Register::AntennaCalibration)?;

        if val & 0x8 != 0 {
            return Err(Error::AntennaCalibration);
        }
        // TODO: investigate and write comment
        as3910.write_register(Register::OperationControl, 0xD0)?;
        as3910.execute_command(Command::Clear)?;

        // Setup interupts maybe?

        Ok(as3910)
    }

    pub fn reset(&mut self) -> Result<(), Error<E, OPE>> {
        self.execute_command(Command::SetDefault)
    }

    /// Sends a Wake UP type A to nearby PICCs
    pub fn wupa<'b>(&mut self) -> Result<Option<AtqA>, Error<E, OPE>> {
        self.execute_command(Command::TransmitWUPA)?;

        self.wait_for_interrupt(50)?;

        let fifo_reg = self.read_register(Register::FIFOStatus)?;

        if fifo_reg >> 2 == 0b00111111 {
            // No PICC in area
            return Ok(None);
        }
        let mut buffer = [0u8; 2];

        self.read_fifo(&mut buffer)?;

        Ok(Some(AtqA { bytes: buffer }))

        // NOTE WUPA is a short frame (7 bits)
        // let fifo_data = self.transceive(&[picc::Command::WUPA as u8], 7, 0)?;
        // if fifo_data.valid_bytes != 2 || fifo_data.valid_bits != 0 {
        //     Err(Error::IncompleteFrame)
        // } else {
        //     Ok(AtqA {
        //         bytes: fifo_data.buffer,
        //     })
        // }
    }

    pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<E, OPE>> {
        let mut buffer = [reg.read_address(), 0];

        self.with_cs_high(|mfr| {
            let buffer = mfr.spi.transfer(&mut buffer).map_err(Error::Spi)?;

            Ok(buffer[1])
        })
    }

    fn read<'b>(&mut self, reg: Register, buffer: &'b mut [u8]) -> Result<&'b [u8], Error<E, OPE>> {
        let byte = reg.read_address();

        self.with_cs_high(move |mfr| {
            mfr.spi.transfer(&mut [byte]).map_err(Error::Spi)?;

            let n = buffer.len();
            for slot in &mut buffer[..n - 1] {
                *slot = mfr.spi.transfer(&mut [byte]).map_err(Error::Spi)?[0];
            }

            buffer[n - 1] = mfr.spi.transfer(&mut [0]).map_err(Error::Spi)?[0];

            Ok(&*buffer)
        })
    }

    fn read_fifo<'b>(&mut self, buffer: &'b mut [u8]) -> Result<&'b [u8], Error<E, OPE>> {
        self.with_cs_high(move |mfr| {
            // initiate fifo read
            mfr.spi.transfer(&mut [0b10111111]).map_err(Error::Spi)?;

            let n = buffer.len();
            for slot in &mut buffer[..n ] {
                *slot = mfr.spi.transfer(&mut [0]).map_err(Error::Spi)?[0];
            }

            Ok(&*buffer)
        })
    }

    fn wait_for_interrupt(&mut self, timeout_in_ms: u16) -> Result<u8, Error<E, OPE>> {
        let mut i = 0;
        loop {
            if self.intr.is_high().map_err(Error::InterruptPin)? {
                return self.read_register(Register::Interrupt);
            }

            if i >= timeout_in_ms {
                break;
            }
            self.delay.delay_ms(1);
            i += 1;
        }

        Err(Error::InterruptTimeout)
    }

    fn execute_command(&mut self, command: Command) -> Result<(), Error<E, OPE>> {
        self.write(&[command.command_pattern()])
    }

    fn write_register(&mut self, reg: Register, val: u8) -> Result<(), Error<E, OPE>> {
        self.write(&[reg.write_address(), val])
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Error<E, OPE>> {
        self.with_cs_high(|mfr| {
            mfr.spi.write(bytes).map_err(Error::Spi)?;

            Ok(())
        })
    }

    fn with_cs_high<F, T>(&mut self, f: F) -> Result<T, Error<E, OPE>>
    where
        F: FnOnce(&mut Self) -> Result<T, Error<E, OPE>>,
    {
        self.cs.set_high().map_err(Error::ChipSelect)?;
        let result = f(self);
        self.cs.set_low().map_err(Error::ChipSelect)?;

        result
    }
}

#[derive(Debug)]
pub enum Error<E, OPE> {
    Spi(E),
    ChipSelect(OPE),
    InterruptPin(OPE),

    /// Set when Calibrate antenna sequence was not able to adjust resonance
    AntennaCalibration,

    InterruptTimeout,
}
