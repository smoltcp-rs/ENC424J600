{ stdenv, fetchFromGitHub, rustPlatform, pkgconfig }:

rustPlatform.buildRustPackage rec {
  version = "0.3.1";
  pname = "itm";

  src = fetchFromGitHub {
    owner = "rust-embedded";
    repo = "itm";
    rev = "v${version}";
    sha256 = "15pa0ydm19vz8p3wairpx3vqzc55rp4lgki143ybgw44sgf8hraj";
  };

  cargoPatches = [ ./itm-cargo-lock.patch ];

  cargoSha256 = "0xzpyafmkp2wvw2332kb1sm2m3lgg6qy6fskcq431d713ilvrkmg";

  nativeBuildInputs = [ pkgconfig ];

  doCheck = false;
}