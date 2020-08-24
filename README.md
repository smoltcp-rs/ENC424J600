# ENC424J600 Driver


## General Instructions

The ENC424J600 Ethernet controller module supports operation in one of the following interfaces: Serial Peripheral Interace (SPI), or Parallel Slave Port (PSP). This Rust library supports the use of SPI for all embedded systems compatible with the Rust [`embedded-hal`](https://crates.io/crates/embedded-hal) crate. 

On ENC424J600, the **INTn/SPISEL** pin is multiplexed with an **interrupt function** (INTn) and an **interface selection function** (SPISEL). During power-up, to select SPI as the interface, INTn/SPISEL needs to latch a logic high for 1-10 us, driven outside of ENC424J600. After ENC424J600 has been initialsed, the same pin can be used to indicate occurrence of interrupt with a logic low, or idling with a logic high, driven by ENC424J600. Therefore, on the microcontroller side, the mode of driving the pin should be chosen by design: it should be **tri-stated** if interrupt is enabled, or **push-pull** otherwise.

To help facilitate the user, we provide a [`nix-shell`](https://nixos.org/nixos/nix-pills/developing-with-nix-shell.html#idm140737320154096) environment and a set of Shell scripts to perform certain tasks, such as creating a ready-to-use [`tmux`](https://github.com/tmux/tmux/wiki) session for debugging an STM32 microcontroller, as well as compiling and running STM32-based examples.


### Instructions for STM32F407 Examples

Currently, the provided examples are for STM32F4xx microcontrollers using the Rust [`stm32f4xx-hal`](https://crates.io/crates/stm32f4xx-hal) crate. These examples assume that the **SPI1** port is connected to the Ethernet module, and the **GPIO PA1** pin is connected to its SPISEL pin. Since no interrupts are involved, GPIO PA1 is configured as a **push-pull** output to only initialise the controller. The program output is logged via ITM stimulus port 0.


## Examples

### Endless Pinging - `tx_stm32f407`

This program demonstrates the Ethernet TX capability on an STM32F407 board. Once loaded and initialised, a specific ping packet is sent (broadcasted) every 100ms. Such a packet has the following properties:

* Destination MAC Address: ff-ff-ff-ff-ff-ff
* Source MAC Address: 08-60-6e-44-42-95
* Destination IP Address: 192.168.1.231
* Source IP Address: 192.168.1.100
* Frame Length in Bytes: 64

#### How-to

1.  Connect your STM32F407 device to the computer. Without changing any code, you may use an STLink V2 debugger.

2.  Create a `tmux` session for debugging:
    ```sh
    $ nix-shell
    [nix-shell]$ run-tmux-env
    ```

3.  When the `tmux` session is ready, on the top-right pane, compile and run the example program:
    ```sh
    [nix-shell]$ tx_stm32f407
    ```

4.  Observe the output on the left pane. If you wish to debug manually, run `run-help` to see the list of all available commands.

#### Expected Output

(Note: the MAC address is an example only.)

```
Eth TX Pinging on STM32-F407 via NIC100/ENC424J600
Ethernet initialised.
MAC Address = 04-91-62-3e-fc-1e
Promiscuous Mode ON
Sending packet (len=64): dest=ff-ff-ff-ff-ff-ff src=08-60-6e-44-42-95 data=08060001 08000604 ...
Packet sent
Sending packet (len=64): dest=ff-ff-ff-ff-ff-ff src=08-60-6e-44-42-95 data=08060001 08000604 ...
Packet sent
...
```


### TCP Echoing & Greeting - `tcp_stm32f407`

This program demonstrates the TCP connectivity using **smoltcp** on an STM32F407 board. Once loaded and initialised, two TCP sockets will be opened on a specific IPv4 address. These sockets are:

1.  **Echoing port - 1234**
    *   This socket receives raw data on all incoming TCP packets on the said port and prints them back on the output.
    *   Note that this socket has a time-out of 10s.
2.  **Greeting port - 4321**
    *   This socket waits for a single incoming TCP packet on the said port, and sends a TCP packet holding a text of greeting on the port.
    *   Note that once a greeting is sent, the socket is closed immediately. Further packets received by the controller are dropped until the initiator disconnects from the port.

#### How-to

1.  Connect your STM32F407 device to the computer. Without changing any code, you may use an STLink V2 debugger.

2.  Create a `tmux` session for debugging:
    ```sh
    $ nix-shell
    [nix-shell]$ run-tmux-env
    ```

3.  When the `tmux` session is ready, on the top-right pane, compile and run the example program. Choose your own IPv4 address and prefix length:
    ```sh
    [nix-shell]$ tcp_stm32f407 <ip> <prefix>
    ```

4.  To test the TCP ports, switch to the bottom-right pane (with <kbd>Ctrl</kbd>+<kbd>B</kbd>, followed by an arrow key) and use utilities like NetCat (`nc`):
    ```sh
    $ nc <ip> <port-number>
    ```
    Multiple instances of Netcat can run to use all the ports simultaneously. Use Ctrl+C to disconnect from the port manually (especially for the greeting port).

5.  Observe the output on the left pane. If you wish to debug manually, run `run-help` to see the list of all available commands.

#### Expected Output

(Note: the IP address, MAC address and timestamps shown below are examples only.)

ITM output at the initial state:
```
Eth TCP Server on STM32-F407 via NIC100/ENC424J600
Ethernet initialised.
Timer initialised.
MAC Address = 04-91-62-3e-fc-1e
TCP sockets will listen at 192.168.1.77/24
[0.0s] Listening to port 1234 for echoing, time-out in 10s
[0.0s] Listening to port 4321 for greeting, please connect to the port
```

The user connects to port 1234 and sends two packets with the following commands:
```sh
$ nc 192.168.1.77 1234
Hello World!
Bye World!
```

The following is appended to the ITM output:
```
[12.950s] Received packet: Ok("Hello world!\n")
[19.0s] Received packet: Ok("Bye world!\n")
```

The user then connects port 4321 with the following command, and immediately receives the following message on their console:
```sh
$ nc 192.168.1.77 4321
Welcome to the server demo for STM32-F407!
```

The following is appended to the ITM output:
```
[24.200s] Greeting sent, socket closed
```

After 10 seconds of the user not sending any more packets on port 1234, the following is appended to the ITM output; meanwhile, the socket is closed by `nc` for the user:
```
[29.0s] Listening to port 1234 for echoing, time-out in 10s
```

The user can now re-connect to port 1234 again.

For port 4321, without closing the port by exiting `nc`, the user keeps sending any packets with the utility. Since the Ethernet controller has already closed the socket, these new packets are dropped and the following error messages are appended to the ITM output:
```
[35.950s] Poll error: Dropped
[36.150s] Poll error: Dropped
...
```

The user then disconnects from port 4321 by exiting `nc`, and the following message is appended to the ITM output:
```
[40.200s] Listening to port 4321 for greeting, please connect to the port
```

The user can now re-connect to port 4321 again.
