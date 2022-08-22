use embedded_hal::{blocking::spi, digital::v2::OutputPin};

#[derive(Debug)]
pub enum WithHighError<E, OPE> {
    SPI(E),
    CS(OPE),
}

pub trait SpiManager: spi::Transfer<u8, Error = Self::SpiError> + spi::Write<u8, Error = Self::SpiError> {
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
