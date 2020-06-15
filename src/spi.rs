use core::fmt;
use stm32f4xx_hal::{
    hal::{
        blocking::spi::Transfer,
        digital::v2::OutputPin,
    },
    time::MegaHertz,
    spi,
};

/// Must use SPI mode cpol=0, cpha=0
pub const SPI_MODE: spi::Mode = spi::Mode {
    polarity: spi::Polarity::IdleLow,
    phase: spi::Phase::CaptureOnFirstTransition,
};
/// Max freq = 14 MHz
pub const SPI_CLOCK: MegaHertz = MegaHertz(14);

/// SPI Opcodes
const RCRU: u8 = 0b00100000;
const WCRU: u8 = 0b00100010;

/// Struct for SPI I/O interface on ENC424J600
/// Note: stm32f4xx_hal::spi's pins include: SCK, MISO, MOSI
pub struct SpiPort<SPI: Transfer<u8>, 
                   NSS: OutputPin> {
    spi: SPI,
    nss: NSS,
}

impl <SPI: Transfer<u8, Error = E>, 
      NSS: OutputPin, 
      E: fmt::Debug> SpiPort<SPI, NSS> {
    // TODO: return as Result()
    pub fn new(spi: SPI, mut nss: NSS) -> Self {
        nss.set_high();

        SpiPort {
            spi, 
            nss
        }
    }

    pub fn read_reg_8b(&mut self, addr: u8) -> Result<u8, SPI::Error> {
        // Using RCRU instruction to read using unbanked (full) address
        let mut r_data = self.transfer(RCRU, addr, 0)?;
        Ok(r_data)
    }

    pub fn read_reg_16b(&mut self, lo_addr: u8) -> Result<u16, SPI::Error> {
        let mut r_data_lo = self.read_reg_8b(lo_addr)?;
        let mut r_data_hi = self.read_reg_8b(lo_addr + 1)?;
        // Combine top and bottom 8-bit to return 16-bit
        Ok(((r_data_hi as u16) << 8) | r_data_lo as u16)
    }

    pub fn write_reg_8b(&mut self, addr: u8, data: u8) -> Result<u8, SPI::Error> {
        // TODO: addr should be separated from w_data
        // Using WCRU instruction to write using unbanked (full) address
        self.transfer(WCRU, addr, data)?;
        Ok(0x01)        // TODO: should not be just 0x01
    }

    fn transfer(&mut self, opcode: u8, addr: u8, data: u8) 
               -> Result<u8, SPI::Error> {
        // TODO: Currently assumes read/write data is only 1-byte
        // Enable chip select
        self.nss.set_low();
        // Start writing to SLAVE
        // TODO: don't just use 3 bytes
        let mut buf: [u8; 3] = [0; 3];
        buf[0] = opcode;
        buf[1] = addr;
        buf[2] = data;
        let result = self.spi.transfer(&mut buf);
        // Disable chip select
        self.nss.set_high();

        match result {
            Ok(_) => Ok(buf[2]),
            Err(e) => Err(e),
        }
    }
}