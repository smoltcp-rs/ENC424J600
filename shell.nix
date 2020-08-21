{ rustChannel ? "nightly" }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  rustPlatform = callPackage ./nix/rustPlatform.nix {};

  itm-tools = callPackage ./nix/itm-tools.nix { inherit rustPlatform; };

  runHelp = writeShellScriptBin "run-help" ''
    echo "[Common Tools]"
    echo "  run-openocd-f4x"
    echo "        - Run OpenOCD in background for STM32F4XX boards."
    echo "  run-itmdemux-follow <stim>"
    echo "        - Read ITM packets and follow. Specify <stim> for the desired ITM Stimulus Port."
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
    echo "[Workspace]"
    echo "  run-tmux-env"
    echo "        - Start a tmux session specially designed for debugging."
    echo "  end-tmux-env"
    echo "        - Safely stop the tmux debugging session."
    echo ""
  '';

  runTmuxEnv = writeShellScriptBin "run-tmux-env" (builtins.readFile ./nix/tmux.sh);
  killTmuxEnv = writeShellScriptBin "end-tmux-env" ''
    # Note: should modify the binary path if targets have changed in ./nix/rustPlatform.nix

    echo 'Stopping GDB if running...'
    # Send SIGINT to GDB
    pkill -2 -f 'gdb -q -x openocd\.gdb target/thumbv7em-none-eabihf/release/examples/'
    # Kill GDB
    pkill -f 'gdb -q -x openocd\.gdb target/thumbv7em-none-eabihf/release/examples/' |

    echo 'Stopping OpenOCD if running...'
    # Kill OpenOCD
    pkill -f 'openocd -f ${openocd}/share/openocd/scripts/interface/stlink-v2.cfg -f ${openocd}/share/openocd/scripts/target/stm32f4x.cfg'
    
    echo 'Stopping tailing ITM outputs...'
    pkill -f 'run-itmdemux-follow'
    pkill -f 'tail -f .\.stim'
    
    echo 'Stopping tmux session...'
    # Kill tmux window
    tmux kill-window -t enc424j600:$USER
  '';

  runOpenOcdF4x = writeShellScriptBin "run-openocd-f4x" ''
    openocd \
    -f ${openocd}/share/openocd/scripts/interface/stlink-v2.cfg \
    -f ${openocd}/share/openocd/scripts/target/stm32f4x.cfg \
    -c init &
    sleep 1
  '';

  runItmDemuxFollow = writeShellScriptBin "run-itmdemux-follow" ''
    export STIM=$1
    if [[ $1 = "" ]]
    then
      echo "Using Stimulus Port 0 by default..."
      export STIM=0
    fi

    # Wait for itm.bin to be created by OpenOCD
    until [ -f itm.bin ]
    do
      sleep 1
    done

    echo 'Stopping running instances of itm-tool port-demux...'
    # Kill running instances of port-demux
    pkill -f 'port-demux' | xargs -r kill

    echo 'Tailing ITM output...'
    port-demux -f itm.bin &
    # Wait for stim file to be created by port-demux
    until [ -f $STIM.stim ]
    do
      sleep 1
    done
    tail -f $STIM.stim
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
    rustc cargo pkgs.gdb pkgs.openocd pkgs.tmux itm-tools 
    runHelp runTmuxEnv killTmuxEnv
    runOpenOcdF4x runItmDemuxFollow
    exTxStm32f407 exTcpStm32f407
  ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;

  shellHook = ''
    echo "Welcome to the nix-shell for running STM32 examples!"
    echo "- run-tmux-env to start a tmux session."
    echo "- run-help to see list of all available commands."
    echo 
  '';
}
