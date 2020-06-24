# ENC424J600 Driver

## Examples

<!---
TODO: Remove the installation/env setup steps in the following how-to;
      only keep example-specific steps.
-->


### Endless Pinging - `tx_stm32f407`

This program demonstrates the Ethernet TX capability on an STM32F407 board connected to an ENC424J600 module via SPI. Once loaded and initialised, a specific ping packet is sent (broadcasted) every 100ms. Such a packet has the following properties:

* Destination MAC Address: ff-ff-ff-ff-ff-ff
* Source MAC Address: 08-60-6e-44-42-95
* Destination IP Address: 192.168.1.231
* Source IP Address: 192.168.1.100
* Frame Length in Bytes: 64

Note that this program uses ITM for logging output.

#### How-to

1.  Prepare the Rust and Cargo environment for building and running the program on STM32F4xx Cortex-M platforms.

2.  Install [`itm`](https://docs.rs/itm/) on Cargo:
    ```
    cargo install itm
    ```
    If necessary, add your installation location (`~/.cargo/bin` by default) to `$PATH`.

3.  Connect your STM32F407 device to the computer. Without changing any code, you may use an STLink V2 debugger. 

4.  Run OpenOCD with the appropriate configuration files (e.g. `interface/stlink-v2.cfg`, `target/stm32f4x.cfg`).

5.  With OpenOCD running, build and run this program:
    ```
    cargo run --release --example=tx_stm32f407 --features=stm32f407
    ```
    Use appropriate GDB commands such as `c` for continuing.

6.  On a separate console window, monitor and observe the ITM output, which is located at the same directory as you started OpenOCD:
    ```
    itmdump -f itm.log -F
    ```


### TCP Echoing & Greeting - `tcp_stm32f407`

This program demonstrates the TCP connectivity using **smoltcp** on an STM32F407 board connected to an ENC424J600 module via SPI. Once loaded and initialised, two TCP sockets will be opened on the IP address 192.168.1.75/24. These sockets are:

1.  **Echoing port - 1234**
    *   This socket receives raw data on all incoming TCP packets on the said port and prints them back on the output.
    *   Note that this socket has a time-out of 10s.
2.  **Greeting port - 4321**
    *   This socket waits for a single incoming TCP packet on the said port, and sends a TCP packet holding a text of greeting on the port.
    *   Note that once a greeting is sent, the port is closed immediately. Further packets received by the controller are dropped until the initiator closes the port.

Note that this program uses ITM for logging output.

#### How-to

1.  Prepare the Rust and Cargo environment for building and running the program on STM32F4xx Cortex-M platforms.

2.  Install [`itm`](https://docs.rs/itm/) on Cargo:
    ```
    cargo install itm
    ```
    If necessary, add your installation location (`~/.cargo/bin` by default) to `$PATH`.

3.  Connect your STM32F407 device to the computer. Without changing any code, you may use an STLink V2 debugger. 

4.  Run OpenOCD with the appropriate configuration files (e.g. `interface/stlink-v2.cfg`, `target/stm32f4x.cfg`).

5.  With OpenOCD running, build and run this program:
    ```
    cargo run --release --example=tcp_stm32f407 --features=stm32f407,smoltcp-phy-all
    ```
    Use appropriate GDB commands such as `c` for continuing.

6.  On a separate console window, monitor and observe the ITM output, which is located at the same directory as you started OpenOCD:
    ```
    itmdump -f itm.log -F
    ```

7.  To test the TCP ports, you may use the Netcat utility (`nc`):
    ```
    nc 192.168.1.75 <port-number>
    ```
    Multiple instances of Netcat can run to use all the ports simultaneously. Use Ctrl+C to close the port manually.
