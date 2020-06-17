use core::fmt;
use stm32f4xx_hal::{
    hal::{
        blocking::spi::Transfer,
        digital::v2::OutputPin,
    },
    time::MegaHertz,
    spi,
};
///
/// FIXME: Move the following to somewhere else
///
use crate::rx;

/// Must use SPI mode cpol=0, cpha=0
pub const SPI_MODE: spi::Mode = spi::Mode {
    polarity: spi::Polarity::IdleLow,
    phase: spi::Phase::CaptureOnFirstTransition,
};
/// Max freq = 14 MHz
pub const SPI_CLOCK: MegaHertz = MegaHertz(14);

/// SPI Opcodes
const RCRU: u8 = 0b0010_0000;
const WCRU: u8 = 0b0010_0010;
const RERXDATA: u8 = 0b0010_1100;   // 8-bit opcode followed by data
const WEGPDATA: u8 = 0b0010_1010;   // 8-bit opcode followed by data

/// SPI Register Mapping
/// Note: PSP interface use different address mapping
// SPI Init Reset Registers
pub const EUDAST: u8 = 0x16;        // 16-bit data
pub const ESTAT: u8 = 0x1a;         // 16-bit data
pub const ECON2: u8 = 0x6e;         // 16-bit data
//
pub const ERXFCON: u8 = 0x34;       // 16-bit data
//
pub const MAADR3: u8 = 0x60;        // 16-bit data
pub const MAADR2: u8 = 0x62;        // 16-bit data
pub const MAADR1: u8 = 0x64;        // 16-bit data
// RX Registers
pub const ERXRDPT: u8 = 0x8a;       // 16-bit data
pub const ERXST: u8 = 0x04;         // 16-bit data
pub const ERXTAIL: u8 = 0x06;       // 16-bit data
pub const EIR: u8 = 0x1c;           // 16-bit data
pub const ECON1: u8 = 0x1e;         // 16-bit data
pub const MAMXFL: u8 = 0x4a;        // 16-bit data
// TX Registers
pub const EGPWRPT: u8 = 0x88;       // 16-bit data
pub const ETXST: u8 = 0x00;         // 16-bit data
pub const ETXSTAT: u8 = 0x12;       // 16-bit data
pub const ETXLEN: u8 = 0x02;        // 16-bit data

/// Struct for SPI I/O interface on ENC424J600
/// Note: stm32f4xx_hal::spi's pins include: SCK, MISO, MOSI
pub struct SpiPort<SPI: Transfer<u8>, 
                   NSS: OutputPin> {
    spi: SPI,
    nss: NSS,
}

pub enum SpiPortError {
    TransferError
}

impl <SPI: Transfer<u8>, 
      NSS: OutputPin> SpiPort<SPI, NSS> {
    // TODO: return as Result()
    pub fn new(spi: SPI, mut nss: NSS) -> Self {
        nss.set_high();
        
        SpiPort {
            spi, 
            nss
        }
    }

    pub fn read_reg_8b(&mut self, addr: u8) -> Result<u8, SpiPortError> {
        // Using RCRU instruction to read using unbanked (full) address
        let mut r_data = self.rw_addr_u8(RCRU, addr, 0)?;
        Ok(r_data)
    }

    pub fn read_reg_16b(&mut self, lo_addr: u8) -> Result<u16, SpiPortError> {
        let mut r_data_lo = self.read_reg_8b(lo_addr)?;
        let mut r_data_hi = self.read_reg_8b(lo_addr + 1)?;
        // Combine top and bottom 8-bit to return 16-bit
        Ok(((r_data_hi as u16) << 8) | r_data_lo as u16)
    }

    // Currently requires manual slicing (buf[1..]) for the data read back
    pub fn read_rxdat<'a>(&mut self, buf: &'a mut [u8], data_length: u32) 
                         -> Result<(), SpiPortError> {
        let r_valid = self.r_n(buf, RERXDATA, data_length)?;
        Ok(r_valid)
    }

    // Currenly requires actual data to be stored in buf[1..] instead of buf[0..]
    // TODO: Maybe better naming?
    pub fn write_txdat<'a>(&mut self, buf: &'a mut [u8], data_length: u32) 
                          -> Result<(), SpiPortError> {
        let w_valid = self.w_n(buf, WEGPDATA, data_length)?;
        Ok(w_valid)
    }

    pub fn write_reg_8b(&mut self, addr: u8, data: u8) -> Result<(), SpiPortError> {
        // TODO: addr should be separated from w_data
        // Using WCRU instruction to write using unbanked (full) address
        self.rw_addr_u8(WCRU, addr, data)?;
        Ok(())
    }

    pub fn write_reg_16b(&mut self, lo_addr: u8, data: u16) -> Result<(), SpiPortError> {
        self.write_reg_8b(lo_addr, (data & 0xff) as u8)?;
        self.write_reg_8b(lo_addr + 1, ((data & 0xff00) >> 8) as u8)?;
        Ok(())
    }

    // TODO: Generalise transfer functions
    // TODO: (Make data read/write as reference to array)
    // Currently requires 1-byte addr, read/write data is only 1-byte
    fn rw_addr_u8(&mut self, opcode: u8, addr: u8, data: u8) 
                 -> Result<u8, SpiPortError> {
        // Enable chip select
        self.nss.set_low();
        // Start writing to SLAVE
        // TODO: don't just use 3 bytes
        let mut buf: [u8; 3] = [0; 3];
        buf[0] = opcode;
        buf[1] = addr;
        buf[2] = data;
        match self.spi.transfer(&mut buf) {
            Ok(_) => {
                // Disable chip select
                self.nss.set_high();
                Ok(buf[2])
            },
            // TODO: Maybe too naive?
            Err(e) => {
                // Disable chip select
                self.nss.set_high();
                Err(SpiPortError::TransferError)
            }
        }
    }

    // TODO: Generalise transfer functions
    // Currently does NOT accept addr, read data is N-byte long 
    // Returns a reference to the data returned
    // Note: buf must be at least (data_length + 1)-byte long
    // TODO: Check and raise error for array size < (data_length + 1)
    fn r_n<'a>(&mut self, buf: &'a mut [u8], opcode: u8, data_length: u32) 
              -> Result<(), SpiPortError> {
        // Enable chip select
        self.nss.set_low();
        // Start writing to SLAVE
        buf[0] = opcode;
        match self.spi.transfer(buf) {
            Ok(_) => {
                // Disable chip select
                self.nss.set_high();
                Ok(())
            },
            // TODO: Maybe too naive?
            Err(e) => {
                // Disable chip select
                self.nss.set_high();
                Err(SpiPortError::TransferError)
            }
        }
    }

    // Note: buf[0] is currently reserved for opcode to overwrite
    // TODO: Actual data should start from buf[0], not buf[1]
    fn w_n<'a>(&mut self, buf: &'a mut [u8], opcode: u8, data_length: u32) 
              -> Result<(), SpiPortError> {
        // Enable chip select
        self.nss.set_low();
        // Start writing to SLAVE
        buf[0] = opcode;
        // TODO: Maybe need to copy data to buf later on
        match self.spi.transfer(buf) {
            Ok(_) => {
                // Disable chip select
                self.nss.set_high();
                Ok(())
            },
            // TODO: Maybe too naive?
            Err(e) => {
                // Disable chip select
                self.nss.set_high();
                Err(SpiPortError::TransferError)
            }
        }
    }
}