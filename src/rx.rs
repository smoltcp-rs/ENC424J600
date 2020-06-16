/// SRAM Addresses
pub const ERXST_DEFAULT: u16 = 0x5340;
pub const ERXTAIL_DEFAULT: u16 = 0x5ffe;
pub const RX_MAX_ADDRESS: u16 = 0x5fff;

/// Max raw frame array size
pub const RAW_FRAME_LENGTH_MAX: usize = 0x1000;
/// Receive Status Vector Length
pub const RSV_LENGTH: usize = 6;

/// Struct for RX Buffer
/// TODO: Should be a singleton
pub struct RxBuffer {
    wrap_addr: u16,
    next_addr: u16,
    tail_addr: u16
}

impl RxBuffer {
    pub fn new() -> Self {
        RxBuffer {
            wrap_addr: ERXST_DEFAULT,
            next_addr: ERXST_DEFAULT,
            tail_addr: ERXTAIL_DEFAULT
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

    pub fn set_tail_addr(&mut self, addr: u16) {
        self.tail_addr = addr;
    }
    pub fn get_tail_addr(& self) -> u16{
        self.tail_addr
    }
}

/// Struct for RX Packet
/// TODO: Generalise MAC addresses
pub struct RxPacket {
    rsv: Rsv,
    frame: [u8; RAW_FRAME_LENGTH_MAX],
    frame_length: usize
}

impl RxPacket {
    pub fn new() -> Self {
        RxPacket {
            rsv: Rsv::new(),
            frame: [0; RAW_FRAME_LENGTH_MAX],
            frame_length: 0
        }
    }

    pub fn write_to_rsv(&mut self, raw_rsv: &[u8]) {
        self.rsv.write_to_rsv(raw_rsv);
    }
    pub fn read_raw_rsv(&self) -> &[u8] {
        self.rsv.read_raw_rsv()
    }

    pub fn update_frame_length(&mut self) {
        self.rsv.set_frame_length();
        self.frame_length = self.rsv.get_frame_length() as usize;
    }

    pub fn get_frame_length(&self) -> usize {
        self.frame_length
    }

    pub fn write_to_frame(&mut self, raw_frame: &[u8]) {
        for i in 0..self.frame_length {
            self.frame[i] = raw_frame[i];
        }
    }

    pub fn get_frame_byte(&self, i: usize) -> u8 {
        self.frame[i]
    }
}

/// Struct for Receive Status Vector
/// See: Table 9-1, ENC424J600 Data Sheet
struct Rsv {
    raw_rsv: [u8; RSV_LENGTH],
    // TODO: Add more definitions
    frame_length: u16
}

impl Rsv {
    fn new() -> Self {
        Rsv {
            raw_rsv: [0; RSV_LENGTH],
            frame_length: 0_u16
        }
    }

    fn write_to_rsv(&mut self, raw_rsv: &[u8]) {
        for i in 0..RSV_LENGTH {
            self.raw_rsv[i] = raw_rsv[i];
        }
    }
    fn read_raw_rsv(&self) -> &[u8] {
        &self.raw_rsv
    }

    fn set_frame_length(&mut self) {
        self.frame_length = (self.raw_rsv[0] as u16) | ((self.raw_rsv[1] as u16) << 8);
    }
    fn get_frame_length(&self) -> u16 {
        self.frame_length
    }
}