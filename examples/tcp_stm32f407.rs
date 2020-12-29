#![no_std]
#![no_main]

extern crate panic_itm;
use cortex_m::{iprintln, iprint};

use embedded_hal::{
    digital::v2::OutputPin,
    blocking::delay::DelayMs
};
use stm32f4xx_hal::{
    rcc::RccExt,
    gpio::GpioExt,
    time::U32Ext,
    stm32::ITM,
    delay::Delay,
    spi::Spi,
    time::Hertz
};
use enc424j600;
use enc424j600::{smoltcp_phy, EthController};

use smoltcp::wire::{
    EthernetAddress, IpAddress, IpCidr, Ipv6Cidr
};
use smoltcp::iface::{NeighborCache, EthernetInterfaceBuilder, EthernetInterface};
use smoltcp::socket::{SocketSet, TcpSocket, TcpSocketBuffer};
use core::str;
use core::fmt::Write;

use rtic::cyccnt::Instant;

///
use stm32f4xx_hal::{
    stm32::SPI1,
    gpio::{
        gpioa::{PA5, PA6, PA7, PA4},
        Alternate, AF5, Output, PushPull
    }
};
type BoosterSpiEth = enc424j600::SpiEth<
    Spi<SPI1, (PA5<Alternate<AF5>>, PA6<Alternate<AF5>>, PA7<Alternate<AF5>>)>,
    PA4<Output<PushPull>>>;

pub struct NetStorage {
    ip_addrs: [IpCidr; 1],
    neighbor_cache: [Option<(IpAddress, smoltcp::iface::Neighbor)>; 8],
}

static mut NET_STORE: NetStorage = NetStorage {
    // Placeholder for the real IP address, which is initialized at runtime.
    ip_addrs: [IpCidr::Ipv6(
        Ipv6Cidr::SOLICITED_NODE_PREFIX,
    )],
    neighbor_cache: [None; 8],
};

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        eth_iface: EthernetInterface<
            'static,
            'static,
            'static,
            smoltcp_phy::SmoltcpDevice<BoosterSpiEth>>,
        itm: ITM
    }

    #[init()]
    fn init(mut c: init::Context) -> init::LateResources {
        c.core.SCB.enable_icache();
        c.core.SCB.enable_dcache(&mut c.core.CPUID);

        // Enable monotonic timer CYCCNT
        c.core.DWT.enable_cycle_counter();
        c.core.DCB.enable_trace();

        let clocks = c.device.RCC.constrain()
            .cfgr
            .sysclk(168.mhz())
            .hclk(168.mhz())
            .pclk1(42.mhz())
            .require_pll48clk()
            .freeze();
        let mut delay = Delay::new(c.core.SYST, clocks);

        // Init ITM
        let mut itm = c.core.ITM;
        let stim0 = &mut itm.stim[0];

        iprintln!(stim0,
            "Eth TCP Server on STM32-F407 via NIC100/ENC424J600");

        // NIC100 / ENC424J600 Set-up
        let spi1 = c.device.SPI1;
        let gpioa = c.device.GPIOA.split();
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
        let eth_iface = {
            let mut spi_eth = {
                let spi_eth_port = Spi::spi1(
                    spi1, (spi1_sck, spi1_miso, spi1_mosi),
                    enc424j600::spi::interfaces::SPI_MODE,
                    Hertz(enc424j600::spi::interfaces::SPI_CLOCK_FREQ),
                    clocks);
                enc424j600::SpiEth::new(spi_eth_port, spi1_nss)
            };

            // Init controller
            match spi_eth.init_dev(&mut delay) {
                Ok(_) => {
                    iprintln!(stim0, "Initializing Ethernet...")
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
            iprintln!(stim0, "Ethernet controller initialized");

            // Init smoltcp interface
            let eth_iface = {
                let device = smoltcp_phy::SmoltcpDevice::new(spi_eth);

                let store = unsafe { &mut NET_STORE };
                store.ip_addrs[0] = IpCidr::new(IpAddress::v4(192, 168, 1, 77), 24);
                let neighbor_cache = NeighborCache::new(&mut store.neighbor_cache[..]);

                EthernetInterfaceBuilder::new(device)
                    .ethernet_addr(EthernetAddress(eth_mac_addr))
                    .neighbor_cache(neighbor_cache)
                    .ip_addrs(&mut store.ip_addrs[..])
                    .finalize()
            };
            iprintln!(stim0, "Ethernet interface initialized");

            eth_iface
        };

        init::LateResources {
            eth_iface,
            itm
        }
    }

    #[idle(resources=[eth_iface, itm])]
    fn idle(c: idle::Context) -> ! {
        let stim0 = &mut c.resources.itm.stim[0];
        let iface = c.resources.eth_iface;

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
        {
            let store = unsafe { &mut NET_STORE };
            iprintln!(stim0,
                "TCP sockets will listen at {}", store.ip_addrs[0].address());
        }

        // Copied / modified from:
        // smoltcp:examples/loopback.rs, examples/server.rs;
        // stm32-eth:examples/ip.rs,
        // git.m-labs.hk/M-Labs/tnetplug
        let mut time = 0u32;
        let mut next_ms = Instant::now();
        use rtic::cyccnt::U32Ext;
        next_ms += 168_000_u32.cycles();
        loop {
            // Poll
            let tick = Instant::now() > next_ms;
            if tick {
                next_ms += 168_000_u32.cycles();
                time += 1;
            }
            let instant = smoltcp::time::Instant::from_millis(time as i64);
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
                    socket.set_timeout(Some(smoltcp::time::Duration::from_millis(10000)));
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
    }

    extern "C" {
        fn EXTI0();
    }
};
