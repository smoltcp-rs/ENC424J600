/// SRAM Addresses
pub const GPBUFST_DEFAULT: u16 = 0x0000;    // Start of General-Purpose SRAM Buffer
pub const GPBUFEN_DEFAULT: u16 = 0x5340;    // End of General-Purpose SRAM Buffer == ERXST default

/// Max raw frame array size
pub const RAW_FRAME_LENGTH_MAX: usize = 0x1000;

/// Struct for TX Buffer
/// TODO: Should be a singleton
pub struct TxBuffer {
    wrap_addr: u16,
    // The following two fields are controlled by firmware
    next_addr: u16,
    tail_addr: u16
}

impl TxBuffer {
    pub fn new() -> Self {
        TxBuffer {
            wrap_addr: GPBUFST_DEFAULT,
            next_addr: GPBUFST_DEFAULT + 1,
            tail_addr: GPBUFST_DEFAULT
        }
    }

    pub fn set_wrap_addr(&mut self, addr: u16) {
        self.wrap_addr = addr;
    }
    pub fn get_wrap_addr(& self) -> u16{
        self.wrap_addr
    }

    pub fn set_next_addr(&mut self, addr: u16) {
        self.next_addr = addr;
    }
    pub fn get_next_addr(& self) -> u16{
        self.next_addr
    }

    // TODO: Need more functions for smoltcp::phy compatibility (maybe?)
}

/// Struct for TX Packet
/// TODO: Generalise MAC addresses
pub struct TxPacket {
    frame: [u8; RAW_FRAME_LENGTH_MAX],
    frame_length: usize
}

impl TxPacket {
    pub fn new() -> Self {
        TxPacket {
            frame: [0; RAW_FRAME_LENGTH_MAX],
            frame_length: 0
        }
    }

    /// Currently, frame data is copied from an external buffer
    pub fn update_frame(&mut self, raw_frame: &[u8], raw_frame_length: usize) {
        self.frame_length = raw_frame_length;
        for i in 0..self.frame_length {
            self.frame[i] = raw_frame[i];
        }
    }
    pub fn copy_from_frame(&self, frame: &mut [u8]) {
        for i in 0..self.frame_length {
            frame[i] = self.frame[i];
        }
    }

    pub fn get_frame_length(&self) -> usize {
        self.frame_length
    }

    /// TODO: Mostly for debugging only?
    pub fn get_frame_byte(&self, i: usize) -> u8 {
        self.frame[i]
    }
}
