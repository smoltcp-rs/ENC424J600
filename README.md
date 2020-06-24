# ENC424J600 Driver


## Examples

### Endless Pinging - `tx_stm32f407`

This program demonstrates the Ethernet TX capability on an STM32F407 board connected to an ENC424J600 module via SPI. Once loaded and initialised, a specific ping packet is sent (broadcasted) every 100ms. Such a packet has the following properties:

* Destination MAC Address: ff-ff-ff-ff-ff-ff
* Source MAC Address: 08-60-6e-44-42-95
* Destination IP Address: 192.168.1.231
* Source IP Address: 192.168.1.100
* Frame Length in Bytes: 64

This program assumes that **GPIO PA1** is connected to SPISEL of the Ethernet module. The program output is logged via ITM.

#### How-to

1.  On a console window, run OpenOCD and debug the example program:
    ```sh
    $ nix-shell
    [nix-shell]$ run-openocd-f4x
    [nix-shell]$ tx_stm32f407
    ```

2.  On a separate console window, run [`itmdump`](https://docs.rs/itm/) to observe the output:
    ```sh
    $ nix-shell
    [nix-shell]$ run-itmdump-follow
    ```

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

This program demonstrates the TCP connectivity using **smoltcp** on an STM32F407 board connected to an ENC424J600 module via SPI. Once loaded and initialised, two TCP sockets will be opened on a specific IPv4 address. These sockets are:

1.  **Echoing port - 1234**
    *   This socket receives raw data on all incoming TCP packets on the said port and prints them back on the output.
    *   Note that this socket has a time-out of 10s.
2.  **Greeting port - 4321**
    *   This socket waits for a single incoming TCP packet on the said port, and sends a TCP packet holding a text of greeting on the port.
    *   Note that once a greeting is sent, the port is closed immediately. Further packets received by the controller are dropped until the initiator closes the port.

This program assumes that **GPIO PA1** is connected to SPISEL of the Ethernet module. The program output is logged via ITM.

#### How-to

1.  On a console window, run OpenOCD and debug the example program. Choose your own IPv4 address and prefix length:
    ```sh
    $ nix-shell
    [nix-shell]$ run-openocd-f4x
    [nix-shell]$ tcp_stm32f407 <ip> <prefix>
    ```

2.  On a separate console window, run [`itmdump`](https://docs.rs/itm/) to observe the output:
    ```sh
    $ nix-shell
    [nix-shell]$ run-itmdump-follow
    ```

3.  To test the TCP ports, open another console window and use utilities like NetCat (`nc`):
    ```sh
    $ nc <ip> <port-number>
    ```
    Multiple instances of Netcat can run to use all the ports simultaneously. Use Ctrl+C to close the port manually (especially for the greeting port).

#### Expected Output

(Note: the IP address, MAC address and timestamps shown below are examples only.)

ITM output at the initial state:
```
Eth TCP Server on STM32-F407 via NIC100/ENC424J600
Ethernet initialised.
Timer initialised.
MAC Address = 04-91-62-3e-fc-1e
TCP sockets will listen at 192.168.1.77/24
[0.0s] Listening to port 1234 for echoing, auto-closing in 10s
[0.0s] Listening to port 4321 for greeting, please open the port
```

The user opens port 1234 and sends two packets with the following commands:
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

The user then opens port 4321 with the following command, and immediately receives the following message on their console:
```sh
$ nc 192.168.1.77 4321
Welcome to the server demo for STM32-F407!
```

The following is appended to the ITM output:
```
[24.200s] Greeting sent, socket closed
```

After 10 seconds of the user not sending any more packets on port 1234, the following is appended to the ITM output; meanwhile, port 1234 is closed by `nc` for the user:
```
[29.0s] Listening to port 1234 for echoing, auto-closing in 10s
```

The user can now re-open port 1234 again.

For port 4321, without closing the port by exiting `nc`, the user keeps sending any packets with the utility. Since the Ethernet controller has already closed the port, these new packets are dropped and the following error messages are appended to the ITM output:
```
[35.950s] Poll error: Dropped
[36.150s] Poll error: Dropped
...
```

The user then closes port 4321 by exiting `nc`, and the following message is appended to the ITM output:
```
[40.200s] Listening to port 4321 for greeting, please open the port
```

The user can now re-open port 4321 again.
