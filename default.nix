{ rustPlatform, pkg-config, systemd }:
rustPlatform.buildRustPackage {
  pname = "nix-sandbox-escape-hatch";
  version = "0.1.0";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ systemd ];
  meta.mainProgram = "nix-sandbox-escape-hatch";
}
