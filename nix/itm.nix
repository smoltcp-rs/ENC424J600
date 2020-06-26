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

  cargoSha256 = "1lgv8nhzbzfw9cl4rhj46a86h9jygz0ih3j4zw5nd1346xgmz7b8";

  nativeBuildInputs = [ pkgconfig ];

  doCheck = false;
}