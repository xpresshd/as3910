#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Puts the AS3910 in default state (same as after power-up)
    SetDefault = 0b000001,
    /// Stops all activities and clears FIFO
    Clear = 0b000010,
    /// Starts a transmit sequence using automatic CRC generation
    TransmitWithCRC = 0b000100,
    /// Starts a transmit sequence without automatic CRC generation
    TransmitWithoutCRC = 0b000101,
    /// Transmits REQA command (ISO-14443A mode only)
    TransmitREQA = 0b000110,
    /// Transmits WUPA command (ISO-14443A mode only)
    TransmitWUPA = 0b000111,
    /// Equivalent to Transmit with CRC with additional RF Collision Avoidance
    NFCTransmitWithInitialRFCollisionAvoidance = 0b001000,
    /// Equivalent to Transmit with CRC with additional RF Collision Avoidance
    NFCTransmitWithResponseRFCollisionAvoidance = 0b001001,
    /// Equivalent to Transmit with CRC with additional RF Collision Avoidance
    NFCTransmitWithResponseRFCollisionAvoidanceWithN0 = 0b001010,
    /// Receive after this command is ignored
    MaskReceiveData = 0b010000,
    /// Receive data following this command is normally processed (this command has priority over internal mask receive timer)
    UnmaskReceiveData = 0b010001,
    /// A/D conversion of signal on AD_IN pin is performed, result is stored in A/D Converter Output Register
    ADConvert = 0b010010,
    /// RF amplitude is measured, result is stored in A/D Converter Output Register
    MeasureRF = 0b010011,
    /// Performs gain reduction based on the current noise level
    Squelch = 0b010100,
    /// Resumes gain settings which were in place before sending Squelch command
    ClearSquelch = 0b010101,
    /// Adjusts supply regulators according to the current supply voltage level
    AdjustRegulators = 0b010110,
    /// Starts sequence which activates the TX, measures the modulation depth and adapts it to comply with the specified modulation depth
    CalibrateModulationDepth = 0b010111,
    /// Starts the sequence to adjust parallel capacitances connected to TRIMx pins so that the antenna LC is in resonance
    CalibrateAntenna = 0b011000,
    /// Measurement of antenna LC tank resonance to determine whether calibration is needed
    CheckAntennaResonance = 0b011001,
    /// Clears RSSI bits and restarts the measurement
    ClearRSSI = 0b011010,
    /// Enter in Transparent mod
    EnterTransparentMode = 0b011100,
}
impl From<Command> for u8 {
    #[inline(always)]
    fn from(variant: Command) -> Self {
        variant as _
    }
}

const C: u8 = (1 << 7) + (1 << 6);

impl Command {
    pub fn command_pattern(&self) -> u8 {
        (*self as u8) | C
    }
}
