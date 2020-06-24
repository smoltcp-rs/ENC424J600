use crate::{
    EthController, rx, tx
};
use core::intrinsics::transmute;
use smoltcp::{
    phy::{Device, DeviceCapabilities, RxToken, TxToken},
    time::Instant,
    Error
};

pub struct SmoltcpDevice<'c> {
    eth_controller: &'c mut dyn EthController<'c>,
    rx_packet_buf: [u8; rx::RAW_FRAME_LENGTH_MAX],
    tx_packet_buf: [u8; tx::RAW_FRAME_LENGTH_MAX]
}

impl<'c> SmoltcpDevice<'c> {
    pub fn new(eth_controller: &'c mut dyn EthController<'c>) -> Self {
        SmoltcpDevice {
            eth_controller,
            rx_packet_buf: [0; rx::RAW_FRAME_LENGTH_MAX],
            tx_packet_buf: [0; tx::RAW_FRAME_LENGTH_MAX]
        }
    }
}

impl<'a, 'c> Device<'a> for SmoltcpDevice<'c> {
    type RxToken = EthRxToken<'a>;
    type TxToken = EthTxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities::default()
    }

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        // Extend self lifetime from 'c to 'a for tokens' access to EthController
        let self_trans = unsafe {
            transmute::<&mut SmoltcpDevice<'c>, &mut SmoltcpDevice<'a>>(&mut *self)
        };
        // Make self_a point to *self that has a lifetime of 'a (extended)
        let self_a = self_trans as *mut SmoltcpDevice<'a>;
        match self_trans.eth_controller.receive_next(false) {
            Ok(rx_packet) => {
                // Write received packet to RX packet buffer
                rx_packet.write_frame_to(&mut self.rx_packet_buf);
                // Construct a RxToken
                let rx_token = EthRxToken {
                    buf: &mut self.rx_packet_buf,
                    len: rx_packet.get_frame_length()
                };
                // Construct a blank TxToken
                let tx_token = EthTxToken {
                    buf: &mut self.tx_packet_buf,
                    dev: self_a
                };
                Some((rx_token, tx_token))
            },
            Err(_) => None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        // Extend self lifetime from 'c to 'a for TxToken's access to EthController
        let self_trans = unsafe {
            transmute::<&mut SmoltcpDevice<'c>, &mut SmoltcpDevice<'a>>(&mut *self)
        };
        // Make self_a point to *self that has a lifetime of 'a (extended)
        let self_a = self_trans as *mut SmoltcpDevice<'a>;
        // Construct a blank TxToken
        let tx_token = EthTxToken {
            buf: &mut self.tx_packet_buf,
            dev: self_a
        };
        Some(tx_token)
    }
}

pub struct EthRxToken<'a> {
    buf: &'a mut [u8],
    len: usize
}

impl<'a> RxToken for EthRxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<R, Error>,
    {
        f(&mut self.buf[..self.len])
    }
}

pub struct EthTxToken<'a> {
    buf: &'a mut [u8],
    dev: *mut SmoltcpDevice<'a>
}

impl<'a> TxToken for EthTxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> Result<R, Error>,
    {
        let result = f(&mut self.buf[..len]);
        // Construct a TxPacket
        let mut tx_packet = tx::TxPacket::new();
        // Update frame length and write frame bytes
        tx_packet.update_frame(&mut self.buf[..len], len);
        // Send the packet as raw
        let eth_controller = unsafe {
            &mut (*self.dev).eth_controller
        };
        match eth_controller.send_raw_packet(&tx_packet) {
            Ok(_) => { result },
            Err(_) => Err(Error::Exhausted)
        }
    }
}
