{ stdenv, fetchFromGitHub, rustPlatform, pkg-config }:

rustPlatform.buildRustPackage rec {
  version = "2019-11-15";
  pname = "itm-tools";

  src = fetchFromGitHub {
    owner = "japaric";
    repo = "itm-tools";
    rev = "e94155e44019d893ac8e6dab51cc282d344ab700";
    sha256 = "19xkjym0i7y52cfhvis49c59nzvgw4906cd8bkz8ka38mbgfqgiy";
  };

  cargoPatches = [ ./itm-tools-cargo-lock.patch ];

  cargoSha256 = "0is702s14pgvd5i2m8aaw3zcsshqrwj97mjgg3wikbc627pagzg7";

  nativeBuildInputs = [ pkg-config ];

  doCheck = false;
}