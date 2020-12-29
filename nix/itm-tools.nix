{ stdenv, fetchFromGitHub, rustPlatform, pkg-config }:

rustPlatform.buildRustPackage rec {
  pname = "itm-tools";
  version = "unstable-2019-11-15";

  src = fetchFromGitHub {
    owner = "japaric";
    repo = pname;
    rev = "e94155e44019d893ac8e6dab51cc282d344ab700";
    sha256 = "19xkjym0i7y52cfhvis49c59nzvgw4906cd8bkz8ka38mbgfqgiy";
  };

  cargoPatches = [ ./itm-tools-cargo-lock.patch ];

  # This hash is computed under nixpkgs 20.09.
  # Older versions nixpkgs would require a different value.
  cargoSha256 = "0rl2ph5igwjl7rwpwcf6afnxly5av7cd6va6wn82lxm606giyq75";

  nativeBuildInputs = [ pkg-config ];

  doCheck = false;
}