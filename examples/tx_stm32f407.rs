#![no_std]
#![no_main]

extern crate panic_itm;
use cortex_m::{iprintln, iprint};

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::DelayMs;
use stm32f4xx_hal::{
    rcc::RccExt,
    gpio::GpioExt,
    time::U32Ext,
    stm32::{CorePeripherals, Peripherals},
    delay::Delay,
    spi::Spi,
    time::Hertz
};
use enc424j600;
use enc424j600::EthController;

#[entry]
fn main() -> ! {
    let mut cp = CorePeripherals::take().unwrap();
    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let dp = Peripherals::take().unwrap();
    let clocks = dp.RCC.constrain()
        .cfgr
        .sysclk(168.mhz())
        .hclk(168.mhz())
        .pclk1(32.mhz())
        .pclk2(64.mhz())
        .freeze();
    let mut delay = Delay::new(cp.SYST, clocks);

    // Init ITM & use Stimulus Port 0
    let mut itm = cp.ITM;
    let stim0 = &mut itm.stim[0];

    iprintln!(stim0,
        "Eth TX Pinging on STM32-F407 via NIC100/ENC424J600");

    // NIC100 / ENC424J600 Set-up
    let spi1 = dp.SPI1;
    let gpioa = dp.GPIOA.split();
    // Mapping: see Table 9, STM32F407ZG Manual
    let spi1_sck = gpioa.pa5.into_alternate_af5();
    let spi1_miso = gpioa.pa6.into_alternate_af5();
    let spi1_mosi = gpioa.pa7.into_alternate_af5();
    let spi1_nss = gpioa.pa4.into_push_pull_output();
    // Map SPISEL: see Table 1, NIC100 Manual
    let mut spisel = gpioa.pa1.into_push_pull_output();
    spisel.set_high().unwrap();
    delay.delay_ms(1_u32);
    spisel.set_low().unwrap();
    // Create SPI1 for HAL
    let spi_eth_port = Spi::spi1(
        spi1, (spi1_sck, spi1_miso, spi1_mosi),
        enc424j600::spi::interfaces::SPI_MODE,
        Hertz(enc424j600::spi::interfaces::SPI_CLOCK_FREQ),
        clocks);
    let mut spi_eth = enc424j600::SpiEth::new(spi_eth_port, spi1_nss);
    // Init
    match spi_eth.init_dev(&mut delay) {
        Ok(_) => {
            iprintln!(stim0, "Ethernet initialized")
        }
        Err(_) => {
            panic!("Ethernet initialization failed!")
        }
    }

    // Read MAC
    let mut eth_mac_addr: [u8; 6] = [0; 6];
    spi_eth.read_from_mac(&mut eth_mac_addr);
    for i in 0..6 {
        let byte = eth_mac_addr[i];
        match i {
            0 => iprint!(stim0, "MAC Address = {:02x}-", byte),
            1..=4 => iprint!(stim0, "{:02x}-", byte),
            5 => iprint!(stim0, "{:02x}\n", byte),
            _ => ()
        };
    }

    // Init Rx/Tx buffers
    spi_eth.init_rxbuf();
    spi_eth.init_txbuf();
    // Testing Eth TX
    let eth_tx_dat: [u8; 64] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x08, 0x60,
        0x6e, 0x44, 0x42, 0x95, 0x08, 0x06, 0x00, 0x01,
        0x08, 0x00, 0x06, 0x04, 0x00, 0x01, 0x08, 0x60,
        0x6e, 0x44, 0x42, 0x95, 0xc0, 0xa8, 0x01, 0x64,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xa8,
        0x01, 0xe7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x69, 0xd0, 0x85, 0x9f
    ];
    loop {
        let mut eth_tx_packet = enc424j600::tx::TxPacket::new();
        eth_tx_packet.update_frame(&eth_tx_dat, 64);
        iprint!(stim0,
            "Sending packet (len={:}): ", eth_tx_packet.get_frame_length());
        for i in 0..20 {
            let byte = eth_tx_packet.get_frame_byte(i);
            match i {
                0 => iprint!(stim0, "dest={:02x}-", byte),
                6 => iprint!(stim0, "src={:02x}-", byte),
                12 => iprint!(stim0, "data={:02x}", byte),
                1..=4 | 7..=10 => iprint!(stim0, "{:02x}-", byte),
                13..=14 | 16..=18 => iprint!(stim0, "{:02x}", byte),
                5 | 11 | 15 => iprint!(stim0, "{:02x} ", byte),
                19 => iprint!(stim0, "{:02x} ...\n", byte),
                _ => ()
            };
        }
        spi_eth.send_raw_packet(&eth_tx_packet);
        iprintln!(stim0, "Packet sent");
        delay.delay_ms(100_u32);
    }
}
