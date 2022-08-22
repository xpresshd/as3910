// #![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate bitflags;

extern crate alloc;

use command::Command;
use embedded_hal as hal;
use hal::blocking::delay;
use hal::blocking::spi;
use hal::digital::v2::InputPin;
use hal::digital::v2::OutputPin;
use register::InterruptFlags;

pub mod command;
mod picc;
pub mod register;

use crate::register::Register;

#[derive(Debug)]
pub enum WithHighError<E, OPE> {
    SPI(E),
    CS(OPE),
}

pub trait SpiWithCustomCS: spi::Transfer<u8, Error = Self::SpiError> + spi::Write<u8, Error = Self::SpiError> {
    type SpiError;
    
    fn with_cs_high<F, T, CS, OPE>(
        &mut self,
        cs: &mut CS,
        f: F,
    ) -> Result<T, WithHighError<Self::SpiError, OPE>>
    where
        F: FnOnce(&mut Self) -> Result<T, Self::SpiError>,
        CS: OutputPin<Error = OPE>;
}

/// Answer To reQuest A
pub struct AtqA {
    pub bytes: [u8; 2],
}

#[derive(Hash, Eq, PartialEq)]
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

#[derive(Hash, Eq, PartialEq)]
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

#[derive(Debug)]
pub struct FifoData<const L: usize> {
    /// The contents of the FIFO buffer
    buffer: [u8; L],
    /// The number of valid bytes in the buffer
    valid_bytes: usize,
    /// The number of valid bits in the last byte
    valid_bits: usize,
}

impl<const L: usize> FifoData<L> {
    /// Copies FIFO data to destination buffer.
    /// Assumes the FIFO data is aligned properly to append directly to the current known bits.
    /// Returns the number of valid bits in the destination buffer after copy.
    pub fn copy_bits_to(&self, dst: &mut [u8], dst_valid_bits: u8) -> u8 {
        if self.valid_bytes == 0 {
            return dst_valid_bits;
        }
        let dst_valid_bytes = dst_valid_bits / 8;
        let dst_valid_last_bits = dst_valid_bits % 8;
        let mask: u8 = 0xFF << dst_valid_last_bits;
        let mut idx = dst_valid_bytes as usize;
        dst[idx] = (self.buffer[0] & mask) | (dst[idx] & !mask);
        idx += 1;
        let len = self.valid_bytes - 1;
        if len > 0 {
            dst[idx..idx + len].copy_from_slice(&self.buffer[1..=len]);
        }
        dst_valid_bits + (len * 8) as u8 + self.valid_bits as u8
    }
}

pub struct AS3910<SPIM, CS, INTR, DELAY> {
    spi_manager: SPIM,
    cs: CS,
    /// Interrupt pin
    intr: INTR,
    delay: DELAY,
}

impl<OPE, CS, INTR, SPIM, DELAY> AS3910<SPIM, CS, INTR, DELAY>
where
    SPIM: SpiWithCustomCS,
    CS: OutputPin<Error = OPE>,
    INTR: InputPin<Error = OPE>,
    DELAY: delay::DelayMs<u16>,
{

    pub fn new(spi_manager: SPIM, cs: CS, intr: INTR, delay: DELAY) -> Result<Self, Error<SPIM::SpiError, OPE>> {
        let mut as3910 = Self {
            spi_manager,
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
        // Enables oscillator and regulator
        // Enables receiver operation
        // Enables RF output
        as3910.write_register(Register::OperationControl, 0xD0)?;

        // PM demodulation
        // as3910.write_register(Register::ConfigurationRegister5, 0b1000_0000)?;
        as3910.execute_command(Command::Clear)?;

        as3910.setup_interrupt_mask(InterruptFlags::END_OF_RECEIVE)?;

        Ok(as3910)
    }

    pub fn reset(&mut self) -> Result<(), Error<SPIM::SpiError, OPE>> {
        self.execute_command(Command::SetDefault)
    }

    /// Sends a REQuest type A to nearby PICCs
    pub fn reqa(&mut self) -> Result<Option<AtqA>, Error<SPIM::SpiError, OPE>> {
        self.execute_command(Command::Clear)?;
        self.write_register(Register::ConfigurationRegister3, 0x80)?;
        self.setup_interrupt_mask(InterruptFlags::END_OF_RECEIVE)?;
        self.execute_command(Command::TransmitREQA)?;

        self.wait_for_interrupt(5)?;

        let fifo_reg = self.read_register(Register::FIFOStatus)?;

        if fifo_reg >> 2 == 0b00111111 {
            // No PICC in area
            return Ok(None);
        }
        let mut buffer = [0u8; 2];

        self.read_fifo(&mut buffer)?;

        Ok(Some(AtqA { bytes: buffer }))
    }

    /// Sends a Wake UP type A to nearby PICCs
    pub fn wupa(&mut self) -> Result<Option<AtqA>, Error<SPIM::SpiError, OPE>> {
        self.setup_interrupt_mask(InterruptFlags::END_OF_RECEIVE)?;
        self.execute_command(Command::TransmitWUPA)?;

        self.wait_for_interrupt(5)?;

        let fifo_reg = self.read_register(Register::FIFOStatus)?;

        if fifo_reg >> 2 == 0b00111111 {
            // No PICC in area
            return Ok(None);
        }
        let mut buffer = [0u8; 2];

        self.read_fifo(&mut buffer)?;

        Ok(Some(AtqA { bytes: buffer }))
    }

    /// Sends command to enter HALT state
    pub fn hlta(&mut self) -> Result<(), Error<SPIM::SpiError, OPE>> {
        // The standard says:
        //   If the PICC responds with any modulation during a period of 1 ms
        //   after the end of the frame containing the HLTA command,
        //   this response shall be interpreted as 'not acknowledge'.
        // We interpret that this way: Only Error::Timeout is a success.
        match self.communicate_to_picc::<0>(&[0x50, 0x00], 0, false, true) {
            Err(Error::InterruptTimeout) => Ok(()),
            Ok(_) => Err(Error::NotAcknowledged),
            Err(e) => Err(e),
        }
    }

    pub fn select(&mut self) -> Result<Uid, Error<SPIM::SpiError, OPE>> {
        let mut cascade_level: u8 = 0;
        let mut uid_bytes: [u8; 10] = [0u8; 10];
        let mut uid_idx: usize = 0;
        let sak = 'cascade: loop {
            let cmd = match cascade_level {
                0 => picc::Command::SelCl1,
                1 => picc::Command::SelCl2,
                2 => picc::Command::SelCl3,
                _ => unreachable!(),
            };
            let mut known_bits = 0;
            let mut tx = [0u8; 9];
            tx[0] = cmd as u8;
            let mut anticollision_cycle_counter = 0;

            'anticollision: loop {
                anticollision_cycle_counter += 1;

                if anticollision_cycle_counter > 32 {
                    return Err(Error::AntiCollisionMaxLoopsReached);
                }
                let tx_last_bits = known_bits % 8;
                let tx_bytes = 2 + known_bits / 8;
                let end = tx_bytes as usize + if tx_last_bits > 0 { 1 } else { 0 };
                tx[1] = (tx_bytes << 4) + tx_last_bits;

                // println!("tx_last_bits {tx_last_bits}");
                // println!("tx_bytes {tx_bytes}");
                // println!("end {end}");
                // println!("tx[1] {}", tx[1]);

                // Tell transceive the only send `tx_last_bits` of the last byte
                // and also to put the first received bit at location `tx_last_bits`.
                // This makes it easier to append the received bits to the uid (in `tx`).
                match self.communicate_to_picc::<5>(&tx[0..end], tx_last_bits, true, false) {
                    Ok(fifo_data) => {
                        fifo_data.copy_bits_to(&mut tx[2..=6], known_bits);
                        // println!("fifo_data {:?}", fifo_data);
                        break 'anticollision;
                    }
                    Err(Error::Collision) => {
                        let coll_reg = self.read_register(Register::Collision)?;

                        let bytes_before_coll = ((coll_reg >> 4) & 0b1111) - 2;
                        let bits_before_coll = (coll_reg >> 1) & 0b111;

                        let coll_pos = bytes_before_coll * 8 + bits_before_coll + 1;
                        // println!("bytes_before_coll {bytes_before_coll}");
                        // println!("bits_before_coll {bits_before_coll}");
                        // println!("coll_pos {coll_pos}");

                        if coll_pos < known_bits || coll_pos > 8 * 9 {
                            // No progress
                            return Err(Error::Collision);
                        }

                        let fifo_data = self.fifo_data::<5>()?;
                        // println!("colission {:?}", fifo_data);

                        fifo_data.copy_bits_to(&mut tx[2..=6], known_bits);
                        known_bits = coll_pos;

                        // Set the bit of collision position to 1
                        let count = known_bits % 8;
                        let check_bit = (known_bits - 1) % 8;
                        let index: usize =
                            1 + (known_bits / 8) as usize + if count != 0 { 1 } else { 0 };
                        // TODO safe check that index is in range
                        tx[index] |= 1 << check_bit;
                    }
                    Err(e) => return Err(e),
                }
            }

            // send select
            tx[1] = 0x70; // NVB: 7 valid bytes
            tx[6] = tx[2] ^ tx[3] ^ tx[4] ^ tx[5]; // BCC

            // TODO check if we send correct based on with crc

            let rx = self.communicate_to_picc::<1>(&tx[0..7], 0, false, true)?;
            // println!("rx {:?}", rx);

            let sak = picc::Sak::from(rx.buffer[0]);

            if !sak.is_complete() {
                uid_bytes[uid_idx..uid_idx + 3].copy_from_slice(&tx[3..6]);
                uid_idx += 3;
                cascade_level += 1;
            } else {
                uid_bytes[uid_idx..uid_idx + 4].copy_from_slice(&tx[2..6]);
                break 'cascade sak;
            }
        };

        match cascade_level {
            0 => Ok(Uid::Single(GenericUid {
                bytes: uid_bytes[0..4].try_into().unwrap(),
                sak,
            })),
            1 => Ok(Uid::Double(GenericUid {
                bytes: uid_bytes[0..7].try_into().unwrap(),
                sak,
            })),
            2 => Ok(Uid::Triple(GenericUid {
                bytes: uid_bytes,
                sak,
            })),
            _ => unreachable!(),
        }
    }

    /// Sends a Wake UP type A to nearby PICCs
    pub fn communicate_to_picc<const RX: usize>(
        &mut self,
        // the data to be sent
        tx_buffer: &[u8],
        // number of bits in the last byte that will be transmitted
        tx_last_bits: u8,
        with_anti_collision: bool,
        with_crc: bool,
    ) -> Result<FifoData<RX>, Error<SPIM::SpiError, OPE>> {
        // println!("Communicate to picc {:x?}", tx_buffer);
        self.setup_interrupt_mask(InterruptFlags::END_OF_RECEIVE)?;

        self.execute_command(Command::Clear)?;

        // TODO: check if we need to set just full bytes or split byte too
        let full_bytes_num = if tx_last_bits == 0 {
            tx_buffer.len()
        } else {
            tx_buffer.len() - 1
        };

        let flags = (full_bytes_num << 6)
            + (((tx_last_bits & 0x7) << 3) as usize)
            + (with_anti_collision as usize);

        // println!("full_bytes_num {full_bytes_num}");
        // println!("flags {flags}");

        self.write_register(Register::NumberOfTransmittedBytes0, flags as u8)?;
        self.write_register(
            Register::NumberOfTransmittedBytes1,
            (full_bytes_num >> 2) as u8,
        )?;

        // Enable AGC (Useful in case the transponder is close to the reader)
        self.write_register(Register::ReceiverConfiguration, 0x80)?;

        if with_crc {
            self.write_register(Register::ConfigurationRegister3, 0x0)?;
        } else {
            self.write_register(Register::ConfigurationRegister3, 0x80)?;
        }

        self.write_fifo(tx_buffer)?;

        if with_crc {
            self.execute_command(Command::TransmitWithCRC)?;
        } else {
            self.execute_command(Command::TransmitWithoutCRC)?;
        }

        let intr = self.wait_for_interrupt(5)?;

        // println!("intr {:?}", intr);

        if intr.contains(InterruptFlags::BIT_COLLISION) {
            return Err(Error::Collision);
        }

        self.fifo_data()

        // let fifo_status = self.read_register(Register::FIFOStatus)?;

        // let num_of_bytes_read = fifo_status >> 2;

        // let mut buffer = [0u8; RX];
        // self.read_fifo(&mut buffer)?;

        // Ok(FifoData {
        //     buffer,
        //     valid_bytes: num_of_bytes_read as usize,
        //     valid_bits: 0,
        // })
    }

    fn fifo_data<const RX: usize>(&mut self) -> Result<FifoData<RX>, Error<SPIM::SpiError, OPE>> {
        let mut buffer = [0u8; RX];
        let mut valid_bytes: usize = 0;
        let mut valid_bits = 0;

        if RX > 0 {
            let fifo_status = self.read_register(Register::FIFOStatus)?;
            // println!("fifo_status {}", fifo_status);

            valid_bytes = (fifo_status >> 2) as usize;
            if valid_bytes > RX {
                return Err(Error::NoRoom);
            }
            if valid_bytes > 0 {
                self.read_fifo(&mut buffer[0..valid_bytes])?;

                // TODO: check
                //valid_bits = (self.read(Register::ControlReg).map_err(Error::Spi)? & 0x07) as usize;
            }
        }

        Ok(FifoData {
            buffer,
            valid_bytes,
            valid_bits,
        })
    }

    pub fn setup_interrupt_mask(&mut self, flags: InterruptFlags) -> Result<u8, Error<SPIM::SpiError, OPE>> {
        // Need to invert bits
        self.write_register(Register::MaskInterrupt, !flags.bits())?;
        // Clear interrupts
        self.read_register(Register::Interrupt)
    }

    pub fn execute_command(&mut self, command: Command) -> Result<(), Error<SPIM::SpiError, OPE>> {
        self.write(&[command.command_pattern()])
    }

    pub fn write_register(&mut self, reg: Register, val: u8) -> Result<(), Error<SPIM::SpiError, OPE>> {
        self.write(&[reg.write_address(), val])
    }

    pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<SPIM::SpiError, OPE>> {
        let mut buffer = [reg.read_address(), 0];

        self.spi_manager.with_cs_high(&mut self.cs,|spi| {
            let buffer = spi.transfer(&mut buffer)?;

            Ok(buffer[1])
        }).map_err(Error::SpiManager)
    }

    fn read<'b>(&mut self, reg: Register, buffer: &'b mut [u8]) -> Result<&'b [u8], Error<SPIM::SpiError, OPE>> {
        let byte = reg.read_address();

        self.spi_manager.with_cs_high(&mut self.cs, move |spi| {
            spi.transfer(&mut [byte])?;

            let n = buffer.len();
            for slot in &mut buffer[..n - 1] {
                *slot = spi.transfer(&mut [byte])?[0];
            }

            buffer[n - 1] = spi.transfer(&mut [0])?[0];

            Ok(&*buffer)
        }).map_err(Error::SpiManager)
    }

    fn read_fifo<'b>(&mut self, buffer: &'b mut [u8]) -> Result<&'b [u8], Error<SPIM::SpiError, OPE>> {
        self.spi_manager.with_cs_high(&mut self.cs, move |spi| {
            // initiate fifo read
            spi.transfer(&mut [0b10111111])?;

            let n = buffer.len();
            for slot in &mut buffer[..n] {
                *slot = spi.transfer(&mut [0])?[0];
            }

            Ok(&*buffer)
        }).map_err(Error::SpiManager)
    }

    fn write_fifo(&mut self, bytes: &[u8]) -> Result<(), Error<SPIM::SpiError, OPE>> {
        self.spi_manager.with_cs_high(&mut self.cs,|spi| {
            // initiate fifo write
            spi.transfer(&mut [0b10000000])?;

            spi.write(bytes)?;

            Ok(())
        }).map_err(Error::SpiManager)
    }

    fn wait_for_interrupt(&mut self, timeout_in_ms: u16) -> Result<InterruptFlags, Error<SPIM::SpiError, OPE>> {
        let mut i = 0;
        loop {
            if self.intr.is_high().map_err(Error::InterruptPin)? {
                return Ok(InterruptFlags::from_bits_truncate(
                    self.read_register(Register::Interrupt)?,
                ));
            }

            if i >= timeout_in_ms {
                break;
            }
            self.delay.delay_ms(1);
            i += 1;
        }

        Err(Error::InterruptTimeout)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Error<SPIM::SpiError, OPE>> {
        self.spi_manager.with_cs_high(&mut self.cs, |spi| {
            spi.write(bytes)?;

            Ok(())
        }).map_err(Error::SpiManager)
    }

}

#[derive(Debug)]
pub enum Error<E, OPE> {
    SpiManager(WithHighError<E, OPE>),
    ChipSelect(OPE),
    InterruptPin(OPE),

    /// Set when Calibrate antenna sequence was not able to adjust resonance
    AntennaCalibration,

    InterruptTimeout,
    NoRoom,
    Collision,
    Proprietary,
    AntiCollisionMaxLoopsReached,
    IncompleteFrame,
    NotAcknowledged,
}
