{ rustChannel ? "nightly" }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  rustPlatform = callPackage ./nix/rustPlatform.nix {};

  # Note: This itm binary is built with a modified version its v0.3.1 source code,
  #       so as to meet the requirements of newer Rust compiler.
  itm = callPackage ./nix/itm.nix { inherit rustPlatform; };

  runHelp = writeShellScriptBin "run-help" ''
    echo "[Common Tools]"
    echo "  run-openocd-f4x"
    echo "        - Run OpenOCD in background for STM32F4XX boards."
    echo "  run-itmdump-follow"
    echo "        - Run itmdump in following mode."
    echo "  run-help"
    echo "        - Display this help message."
    echo ""
    echo "[Examples]"
    echo "  tx_stm32f407"
    echo "        - Run tx_stm32f407 example."
    echo "  tcp_stm32f407 <ip> <pref>"
    echo "        - Run tcp_stm32f407 example with the IPv4"
    echo "          address <ip> (dot-separated) and prefix length <pref>."
    echo ""
  '';

  runOpenOcdF4x = writeShellScriptBin "run-openocd-f4x" ''
    openocd \
    -f ${openocd}/share/openocd/scripts/interface/stlink-v2.cfg \
    -f ${openocd}/share/openocd/scripts/target/stm32f4x.cfg \
    -c init &
    sleep 1
  '';

  runItmDumpFollow = writeShellScriptBin "run-itmdump-follow" ''
    itmdump -f itm.log -F
  '';

  # Examples
  exTxStm32f407 = writeShellScriptBin "tx_stm32f407" ''
    cargo run --release --example=tx_stm32f407 --features=stm32f407
  '';
  exTcpStm32f407 = writeShellScriptBin "tcp_stm32f407" ''
    if [[ $1 = "" ]] || [[ $2 = "" ]]
    then
      echo "Arguments <ip> or <pref> are missing."
      exit
    fi
    touch ./examples/tcp_stm32f407.rs
    export ENC424J600_TCP_IP=$1
    export ENC424J600_TCP_PREF=$2
    cargo run --release --example=tcp_stm32f407 --features=stm32f407,smoltcp-phy-all
  '';
in
stdenv.mkDerivation {
  name = "enc424j600-stm32-env";
  buildInputs = with rustPlatform.rust; [
    rustc cargo pkgs.gdb pkgs.openocd itm
    runHelp runOpenOcdF4x runItmDumpFollow
    exTxStm32f407 exTcpStm32f407
  ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;

  shellHook = ''
    echo "Welcome to the nix-shell for running STM32 examples!"
    echo ""
    run-help
  '';
}
