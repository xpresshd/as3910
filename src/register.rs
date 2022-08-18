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

const R: u8 = 1 << 7;
const W: u8 = 0 << 7;

impl Register {
    fn read_address(&self) -> u8 {
        ((*self as u8) << 1) | R
    }

    fn write_address(&self) -> u8 {
        ((*self as u8) << 1) | W
    }
}
