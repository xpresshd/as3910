#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Register {
    ModeDefinition = 0x00,
    OperationControl = 0x01,

    ConfigurationRegister2 = 0x02,
    /// SO-14443A and NFC
    ConfigurationRegister3 = 0x03,
    /// SO-14443B
    ConfigurationRegister4 = 0x04,
    ConfigurationRegister5 = 0x05,

    ReceiverConfiguration = 0x06,
    MaskInterrupt = 0x07,
    Interrupt = 0x08,
    FIFOStatus = 0x09,
    /// For ISO-14443A only
    Collision = 0x0A,
    NumberOfTransmittedBytes0 = 0x0B,
    NumberOfTransmittedBytes1 = 0x0C,
    ADConverterOutput = 0x0D,
    AntennaCalibration = 0x0E,
    ExternalTrim = 0x0F,

    ModularDepthDefinition = 0x10,
    ModularDepthDisplay = 0x11,
    AntennaDriverAMModulatedLevelDefinition = 0x12,
    AntennaDriverNonModulatedLevelDefinition = 0x13,
    NFCIPFieldDetectionThreshold  = 0x14,

    RegulatorsDisplay  = 0x15,
    RegulatedVoltageDefinition  = 0x16,
    ReceiverStateDisplay  = 0x17,
}

impl From<Register> for u8 {
    #[inline(always)]
    fn from(variant: Register) -> Self {
        variant as _
    }
}

const R: u8 = 1 << 6;
const W: u8 = 0 << 6;

impl Register {
    pub fn read_address(&self) -> u8 {
        (*self as u8) | R
    }

    pub fn write_address(&self) -> u8 {
        (*self as u8) | W
    }
}

bitflags! {
    pub struct InterruptFlags: u8 {
        const BIT_COLLISION = 0b0000_0001;
        const CRC_ERROR = 0b0000_0010;
        const RECEIVE_DATA_CODING_ERROR = 0b000_0100;
        const END_OF_TRANSMISSION = 0b000_1000;
        const END_OF_RECEIVE = 0b0001_0000;
        const FIFO_WATER_LEVEL = 0b0010_0000;
        const NFC_EVENT = 0b0100_0000;
        const OSCILLATOR_FREQUENCY_STABLE = 0b1000_0000;

        const ALL = 0b1111_1111;
    }
}
