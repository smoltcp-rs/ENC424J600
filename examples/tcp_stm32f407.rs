#![no_std]
#![no_main]

use core::env;

extern crate panic_itm;
use cortex_m::iprintln;

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use stm32f4xx_hal::{
    rcc::RccExt,
    gpio::GpioExt,
    time::U32Ext,
    stm32::{CorePeripherals, Peripherals},
    spi::Spi
};
use enc424j600;
use enc424j600::EthController;
use enc424j600::smoltcp_phy;

use smoltcp::wire::{
    EthernetAddress, IpAddress, IpCidr
};
use smoltcp::iface::{NeighborCache, EthernetInterfaceBuilder};
use smoltcp::socket::{SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::{Instant, Duration};
use core::str;
use core::fmt::Write;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::exception;
use stm32f4xx_hal::{
    rcc::Clocks,
    time::MilliSeconds,
    timer::{Timer, Event as TimerEvent},
    stm32::SYST,
};
/// Rate in Hz
const TIMER_RATE: u32 = 20;
/// Interval duration in milliseconds
const TIMER_DELTA: u32 = 1000 / TIMER_RATE;
/// Elapsed time in milliseconds
static TIMER_MS: Mutex<RefCell<u32>> = Mutex::new(RefCell::new(0));

/// Setup SysTick exception
fn timer_setup(syst: SYST, clocks: Clocks) {
    let mut timer = Timer::syst(syst, TIMER_RATE.hz(), clocks);
    timer.listen(TimerEvent::TimeOut);
}

/// SysTick exception (Timer)
#[exception]
fn SysTick() {
    cortex_m::interrupt::free(|cs| {
        *TIMER_MS.borrow(cs)
            .borrow_mut() += TIMER_DELTA;
    });
}

/// Obtain current time in milliseconds
pub fn timer_now() -> MilliSeconds {
    let ms = cortex_m::interrupt::free(|cs| {
        *TIMER_MS.borrow(cs)
            .borrow()
    });
    ms.ms()
}

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

    // Init ITM & use Stimulus Port 0
    let mut itm = cp.ITM;
    let stim0 = &mut itm.stim[0];

    iprintln!(stim0, 
        "Eth TCP Server on STM32-F407 via NIC100/ENC424J600");

    // Get IP address from args
    let arg_ip_raw = env!("ENC424J600_TCP_IP");
    let mut arg_ip_str = arg_ip_raw.split('.');
    let mut arg_ip: [u8; 4] = [0; 4];
    for i in 0..4 {
        match arg_ip_str.next() {
            Some(x) => {
                match x.parse() {
                    Ok(x_) => { arg_ip[i] = x_ },
                    Err(_) => { panic!("IPv4 address invalid!") }
                }
            },
            None => { panic!("IPv4 address invalid!") }
        }
    }
    // Get IP prefix length from args
    let arg_ip_pref_raw = env!("ENC424J600_TCP_PREF");
    let mut arg_ip_pref: u8 = 0;
    match arg_ip_pref_raw.parse() {
        Ok(x) => { arg_ip_pref = x },
        Err(_) => { panic!("IP prefix length invalid!") }
    }

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
    spisel.set_low().unwrap();
    // Create SPI1 for HAL
    let spi_eth_port = Spi::spi1(
        spi1, (spi1_sck, spi1_miso, spi1_mosi), 
        enc424j600::spi::interfaces::SPI_MODE, 
        enc424j600::spi::interfaces::SPI_CLOCK.into(),
        clocks);
    let mut spi_eth = enc424j600::SpiEth::new(spi_eth_port, spi1_nss);
    // Init
    match spi_eth.init_dev() {
        Ok(_) => {
            iprintln!(stim0, "Ethernet initialised.")
        }
        Err(_) => {
            panic!("Ethernet initialisation Failed!")
        }
    }

    // Setup SysTick
    // Reference to stm32-eth:examples/ip.rs
    timer_setup(cp.SYST, clocks);
    iprintln!(stim0, "Timer initialised.");

    // Read MAC
    let mut eth_mac_addr: [u8; 6] = [0; 6];
    spi_eth.read_from_mac(&mut eth_mac_addr);
    iprintln!(stim0, 
        "MAC Address = {:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}", 
        eth_mac_addr[0], eth_mac_addr[1], 
        eth_mac_addr[2], eth_mac_addr[3], 
        eth_mac_addr[4], eth_mac_addr[5]);

    // Init Rx/Tx buffers
    spi_eth.init_rxbuf();
    spi_eth.init_txbuf();

    // Copied / modified from smoltcp:
    // examples/loopback.rs, examples/multicast.rs
    let device = smoltcp_phy::SmoltcpDevice::new(&mut spi_eth);
    let mut neighbor_cache_entries = [None; 16];
    let mut neighbor_cache = NeighborCache::new(&mut neighbor_cache_entries[..]);
    let ip_addr = IpCidr::new(IpAddress::v4(
        arg_ip[0], arg_ip[1], arg_ip[2], arg_ip[3]), arg_ip_pref);
    let mut ip_addrs = [ip_addr];
    let mut iface = EthernetInterfaceBuilder::new(device)
            .ethernet_addr(EthernetAddress(eth_mac_addr))
            .neighbor_cache(neighbor_cache)
            .ip_addrs(&mut ip_addrs[..])
            .finalize();

    // Copied / modified from smoltcp:
    // examples/loopback.rs
    let echo_socket = {
        static mut TCP_SERVER_RX_DATA: [u8; 1024] = [0; 1024];
        static mut TCP_SERVER_TX_DATA: [u8; 1024] = [0; 1024];
        let tcp_rx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_SERVER_RX_DATA[..] });
        let tcp_tx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_SERVER_TX_DATA[..] });
        TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer)
    };
    let greet_socket = {
        static mut TCP_SERVER_RX_DATA: [u8; 256] = [0; 256];
        static mut TCP_SERVER_TX_DATA: [u8; 256] = [0; 256];
        let tcp_rx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_SERVER_RX_DATA[..] });
        let tcp_tx_buffer = TcpSocketBuffer::new(unsafe { &mut TCP_SERVER_TX_DATA[..] });
        TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer)
    };
    let mut socket_set_entries = [None, None];
    let mut socket_set = SocketSet::new(&mut socket_set_entries[..]);
    let echo_handle = socket_set.add(echo_socket);
    let greet_handle = socket_set.add(greet_socket);
    iprintln!(stim0, 
        "TCP sockets will listen at {}", ip_addr);

    // Copied / modified from: 
    // smoltcp:examples/loopback.rs, examples/server.rs;
    // stm32-eth:examples/ip.rs,
    // git.m-labs.hk/M-Labs/tnetplug
    loop {
        let now = timer_now().0;
        let instant = Instant::from_millis(now as i64);
        match iface.poll(&mut socket_set, instant) {
            Ok(_) => {
            },
            Err(e) => {
                iprintln!(stim0, "[{}] Poll error: {:?}", instant, e)
            }
        }
        // Control the "echoing" socket (:1234)
        {
            let mut socket = socket_set.get::<TcpSocket>(echo_handle);
            if !socket.is_open() {
                iprintln!(stim0, 
                    "[{}] Listening to port 1234 for echoing, time-out in 10s", instant);
                socket.listen(1234).unwrap();
                socket.set_timeout(Some(Duration::from_millis(10000)));
            }
            if socket.can_recv() {
                iprintln!(stim0, 
                "[{}] Received packet: {:?}", instant, socket.recv(|buffer| {
                    (buffer.len(), str::from_utf8(buffer).unwrap())
                }));
            }
        }
        // Control the "greeting" socket (:4321)
        {
            let mut socket = socket_set.get::<TcpSocket>(greet_handle);
            if !socket.is_open() {
                iprintln!(stim0, 
                    "[{}] Listening to port 4321 for greeting, \
                    please connect to the port", instant);
                socket.listen(4321).unwrap();
            }

            if socket.can_send() {
                let greeting = "Welcome to the server demo for STM32-F407!";
                write!(socket, "{}\n", greeting).unwrap();
                iprintln!(stim0, 
                    "[{}] Greeting sent, socket closed", instant);
                socket.close();
            }
        }
        
    }

    unreachable!()
}
