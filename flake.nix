{
  description = "sendspin-rs - Rust client library for the Sendspin protocol";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      src = craneLib.cleanCargoSource ./.;

      commonArgs = {
        inherit src;
        strictDeps = true;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs;
          lib.optionals stdenv.hostPlatform.isLinux [
            alsa-lib
          ]
          ++ lib.optionals stdenv.hostPlatform.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreAudio
          ];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      sendspin = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) pname version;
          cargoExtraArgs = "--bin sendspin";
          postInstall = ''
            target_dir="''${CARGO_TARGET_DIR:-target}"
            profile="''${CRANE_PROFILE:-release}"
            bin_path="$target_dir/''\${CARGO_BUILD_TARGET:+$CARGO_BUILD_TARGET/}$profile/sendspin"

            if [ -f "$bin_path" ]; then
              install -Dm755 "$bin_path" "$out/bin/sendspin"
            fi
          '';
          meta.mainProgram = "sendspin";
        });
    in {
      formatter = pkgs.alejandra;

      checks = {
        inherit sendspin;

        clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        fmt = craneLib.cargoFmt {
          inherit src;
        };

        nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestExtraArgs = "--no-tests=pass";
          });
      };

      packages = {
        default = sendspin;
        sendspin = sendspin;
      };

      apps.default = flake-utils.lib.mkApp {
        drv = sendspin;
      };

      devShells.default = craneLib.devShell {
        inherit (commonArgs) nativeBuildInputs buildInputs;
        inputsFrom = [sendspin];

        packages =
          with pkgs;
          [
            cargo-nextest
            cargo-watch
            rust-analyzer
          ]
          ++ commonArgs.nativeBuildInputs
          ++ commonArgs.buildInputs;

        env.RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      };
    });
}
