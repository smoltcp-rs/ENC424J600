{ recurseIntoAttrs, stdenv, lib,
  makeRustPlatform, defaultCrateOverrides, 
  fetchurl, patchelf,
  rustManifest ? ./channel-rust-nightly.toml
}:

let
  targets = [
    "thumbv7em-none-eabihf"   # For ARM Cortex-M4 or M7 w/ FPU support
  ];
  rustChannel =
    lib.rustLib.fromManifestFile rustManifest {
      inherit stdenv fetchurl patchelf;
    };
  rust =
    rustChannel.rust.override {
      inherit targets;
    };

in
makeRustPlatform {
  rustc = rust;
  cargo = rust;
}
