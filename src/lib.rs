#![no_std]

pub mod spi;
use embedded_hal::{
    blocking::{
        spi::Transfer,
        delay::DelayUs,
    },
    digital::v2::OutputPin,
};

pub mod rx;
pub mod tx;

#[cfg(feature="smoltcp")]
pub mod smoltcp_phy;

/// Max raw frame array size
pub const RAW_FRAME_LENGTH_MAX: usize = 0x1000;

pub trait EthController {
    fn init_dev(&mut self, delay: &mut impl DelayUs<u16>) -> Result<(), EthControllerError>;
    fn init_rxbuf(&mut self) -> Result<(), EthControllerError>;
    fn init_txbuf(&mut self) -> Result<(), EthControllerError>;
    fn receive_next(&mut self, is_poll: bool) -> Result<rx::RxPacket, EthControllerError>;
    fn send_raw_packet(&mut self, packet: &tx::TxPacket) -> Result<(), EthControllerError>;
    fn set_promiscuous(&mut self) -> Result<(), EthControllerError>;
    fn read_from_mac(&mut self, mac: &mut [u8]) -> Result<(), EthControllerError>;
}

/// TODO: Improve these error types
pub enum EthControllerError {
    SpiPortError,
    GeneralError,
    // TODO: Better name?
    NoRxPacketError
}

impl From<spi::SpiPortError> for EthControllerError {
    fn from(_: spi::SpiPortError) -> EthControllerError {
        EthControllerError::SpiPortError
    }
}

/// Ethernet controller using SPI interface
pub struct SpiEth<SPI: Transfer<u8>,
                  NSS: OutputPin> {
    spi_port: spi::SpiPort<SPI, NSS>,
    rx_buf: rx::RxBuffer,
    tx_buf: tx::TxBuffer
}

impl <SPI: Transfer<u8>,
      NSS: OutputPin> SpiEth<SPI, NSS> {
    pub fn new(spi: SPI, nss: NSS) -> Self {
        SpiEth {
            spi_port: spi::SpiPort::new(spi, nss),
            rx_buf: rx::RxBuffer::new(),
            tx_buf: tx::TxBuffer::new()
        }
    }
}

impl <SPI: Transfer<u8>,
      NSS: OutputPin> EthController for SpiEth<SPI, NSS> {
    fn init_dev(&mut self, delay: &mut impl DelayUs<u16>) -> Result<(), EthControllerError> {
        // Write 0x1234 to EUDAST
        self.spi_port.write_reg_16b(spi::addrs::EUDAST, 0x1234)?;
        // Verify that EUDAST is 0x1234
        let mut eudast = self.spi_port.read_reg_16b(spi::addrs::EUDAST)?;
        if eudast != 0x1234 {
            return Err(EthControllerError::GeneralError)
        }
        // Poll CLKRDY (ESTAT<12>) to check if it is set
        loop {
            let estat = self.spi_port.read_reg_16b(spi::addrs::ESTAT)?;
            if estat & 0x1000 == 0x1000 { break }
        }
        // Set ETHRST (ECON2<4>) to 1
        let econ2 = self.spi_port.read_reg_8b(spi::addrs::ECON2)?;
        self.spi_port.write_reg_8b(spi::addrs::ECON2, 0x10 | (econ2 & 0b11101111))?;
        // Wait for 25us
        delay.delay_us(25_u16);
        // Verify that EUDAST is 0x0000
        eudast = self.spi_port.read_reg_16b(spi::addrs::EUDAST)?;
        if eudast != 0x0000 {
            return Err(EthControllerError::GeneralError)
        }
        // Wait for 256us
        delay.delay_us(256_u16);
        Ok(())
    }

    fn init_rxbuf(&mut self) -> Result<(), EthControllerError> {
        // Set ERXST pointer
        self.spi_port.write_reg_16b(spi::addrs::ERXST, self.rx_buf.get_wrap_addr())?;
        // Set ERXTAIL pointer
        self.spi_port.write_reg_16b(spi::addrs::ERXTAIL, self.rx_buf.get_tail_addr())?;
        // Set MAMXFL to maximum number of bytes in each accepted packet
        self.spi_port.write_reg_16b(spi::addrs::MAMXFL, RAW_FRAME_LENGTH_MAX as u16)?;
        // Enable RXEN (ECON1<0>)
        let econ1 = self.spi_port.read_reg_16b(spi::addrs::ECON1)?;
        self.spi_port.write_reg_16b(spi::addrs::ECON1, 0x1 | (econ1 & 0xfffe))?;
        Ok(())
    }

    fn init_txbuf(&mut self) -> Result<(), EthControllerError> {
        // Set EGPWRPT pointer
        self.spi_port.write_reg_16b(spi::addrs::EGPWRPT, 0x0000)?;
        Ok(())
    }

    /// Receive the next packet and return it
    /// Set is_poll to true for returning until PKTIF is set;
    /// Set is_poll to false for returning Err when PKTIF is not set
    fn receive_next(&mut self, is_poll: bool) -> Result<rx::RxPacket, EthControllerError> {
        // Poll PKTIF (EIR<4>) to check if it is set
        loop {
            let eir = self.spi_port.read_reg_16b(spi::addrs::EIR)?;
            if eir & 0x40 == 0x40 { break }
            if !is_poll {
                return Err(EthControllerError::NoRxPacketError)
            }
        }
        // Set ERXRDPT pointer to next_addr
        self.spi_port.write_reg_16b(spi::addrs::ERXRDPT, self.rx_buf.get_next_addr())?;
        // Read 2 bytes to update next_addr
        let mut next_addr_buf = [0; 3];
        self.spi_port.read_rxdat(&mut next_addr_buf, 2)?;
        self.rx_buf.set_next_addr((next_addr_buf[1] as u16) | ((next_addr_buf[2] as u16) << 8));
        // Read 6 bytes to update rsv
        let mut rsv_buf = [0; 7];
        self.spi_port.read_rxdat(&mut rsv_buf, 6)?;
        // Construct an RxPacket
        // TODO: can we directly assign to fields instead of using functions?
        let mut rx_packet = rx::RxPacket::new();
        // Get and update frame length
        rx_packet.write_to_rsv(&rsv_buf[1..]);
        rx_packet.update_frame_length();
        // Read frame bytes
        let mut frame_buf = [0; RAW_FRAME_LENGTH_MAX];
        self.spi_port.read_rxdat(&mut frame_buf, rx_packet.get_frame_length())?;
        rx_packet.copy_frame_from(&frame_buf[1..]);
        // Set ERXTAIL pointer to (next_addr - 2)
        if self.rx_buf.get_next_addr() > rx::ERXST_DEFAULT {
            self.spi_port.write_reg_16b(spi::addrs::ERXTAIL, self.rx_buf.get_next_addr() - 2)?;
        } else {
            self.spi_port.write_reg_16b(spi::addrs::ERXTAIL, rx::RX_MAX_ADDRESS - 1)?;
        }
        // Set PKTDEC (ECON1<88>) to decrement PKTCNT
        let econ1_hi = self.spi_port.read_reg_8b(spi::addrs::ECON1 + 1)?;
        self.spi_port.write_reg_8b(spi::addrs::ECON1 + 1, 0x01 | (econ1_hi & 0xfe))?;
        // Return the RxPacket
        Ok(rx_packet)
    }

    /// Send an established packet
    fn send_raw_packet(&mut self, packet: &tx::TxPacket) -> Result<(), EthControllerError> {
        // Set EGPWRPT pointer to next_addr
        self.spi_port.write_reg_16b(spi::addrs::EGPWRPT, self.tx_buf.get_next_addr())?;
        // Copy packet data to SRAM Buffer
        // 1-byte Opcode is included
        let mut txdat_buf: [u8; RAW_FRAME_LENGTH_MAX + 1] = [0; RAW_FRAME_LENGTH_MAX + 1];
        packet.write_frame_to(&mut txdat_buf[1..]);
        self.spi_port.write_txdat(&mut txdat_buf, packet.get_frame_length())?;
        // Set ETXST to packet start address
        self.spi_port.write_reg_16b(spi::addrs::ETXST, self.tx_buf.get_next_addr())?;
        // Set ETXLEN to packet length
        self.spi_port.write_reg_16b(spi::addrs::ETXLEN, packet.get_frame_length() as u16)?;
        // Set TXRTS (ECON1<1>) to start transmission
        let mut econ1_lo = self.spi_port.read_reg_8b(spi::addrs::ECON1)?;
        self.spi_port.write_reg_8b(spi::addrs::ECON1, 0x02 | (econ1_lo & 0xfd))?;
        // Poll TXRTS (ECON1<1>) to check if it is reset
        loop {
            econ1_lo = self.spi_port.read_reg_8b(spi::addrs::ECON1)?;
            if econ1_lo & 0x02 == 0 { break }
        }
        // TODO: Read ETXSTAT to understand Ethernet transmission status
        // (See: Register 9-2, ENC424J600 Data Sheet)
        // Update TX buffer start address
        self.tx_buf.set_next_addr((self.tx_buf.get_next_addr() + packet.get_frame_length() as u16) %
            tx::GPBUFEN_DEFAULT);
        Ok(())
    }

    /// Set controller to Promiscuous Mode
    fn set_promiscuous(&mut self) -> Result<(), EthControllerError> {
        // From Section 10.12, ENC424J600 Data Sheet:
        // "To accept all incoming frames regardless of content (Promiscuous mode),
        // set the CRCEN, RUNTEN, UCEN, NOTMEEN and MCEN bits."
        let erxfcon_lo = self.spi_port.read_reg_8b(spi::addrs::ERXFCON)?;
        self.spi_port.write_reg_8b(spi::addrs::ERXFCON, 0b0101_1110 | (erxfcon_lo & 0b1010_0001))?;
        Ok(())
    }

    /// Read MAC to [u8; 6]
    fn read_from_mac(&mut self, mac: &mut [u8]) -> Result<(), EthControllerError> {
        mac[0] = self.spi_port.read_reg_8b(spi::addrs::MAADR1)?;
        mac[1] = self.spi_port.read_reg_8b(spi::addrs::MAADR1 + 1)?;
        mac[2] = self.spi_port.read_reg_8b(spi::addrs::MAADR2)?;
        mac[3] = self.spi_port.read_reg_8b(spi::addrs::MAADR2 + 1)?;
        mac[4] = self.spi_port.read_reg_8b(spi::addrs::MAADR3)?;
        mac[5] = self.spi_port.read_reg_8b(spi::addrs::MAADR3 + 1)?;
        Ok(())
    }
}
