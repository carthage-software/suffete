{
  description = "Suffete - devshell using rustup (stable 1.95.0 + nightly)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        isDarwin = pkgs.stdenv.isDarwin;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustup
            pkgs.rust-analyzer
            pkgs.pkg-config
            pkgs.openssl
            pkgs.just
          ] ++ pkgs.lib.optionals isDarwin [
            pkgs.libiconv
          ];

          NIX_LDFLAGS = pkgs.lib.optionalString isDarwin ''
            -framework Security -framework SystemConfiguration
          '';

          OPENSSL_NO_VENDOR = 1;
          RUSTFLAGS = "-C debuginfo=1";
          CARGO_TERM_COLOR = "always";
          CARGO_INCREMENTAL = "1";

          shellHook = ''
            export PATH="$HOME/.cargo/bin:$PATH"
            if ! command -v rustc >/dev/null 2>&1; then
              rustup toolchain install 1.95.0 --profile minimal
              rustup toolchain install nightly --profile minimal
              rustup default 1.95.0
            fi
            echo "[suffete] rustc:   $(rustc --version)"
            echo "[suffete] nightly: $(rustup run nightly rustc --version)"
            echo "[suffete] Run: just build | just test | just check | just fix | just bench"
          '';
        };
      });
}
