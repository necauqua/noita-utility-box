{
  description = "Noita Utility Box - a collection of memory-reading utilities for the game Noita";

  # I don't understand like a quarter of this flake, haven't slept for the last 48 hours lol
  # Adapted from https://gitlab.com/mud-rs/milk/-/blob/56f03874c577261f2c520461aebddd47c649ea30/flake.nix
  # But suprisingly, it worked quickly, not complaining

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-substituters = [ "https://necauqua.cachix.org" ];
    extra-trusted-public-keys = [ "necauqua.cachix.org-1:XG5McOG0XwQ9kayUuEiEn0cPoLAMvc2TVs3fXqv/7Uc=" ];
  };

  outputs = { self, nixpkgs, naersk, flake-utils, fenix }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      name = cargoToml.package.name;
      version = cargoToml.package.version;
    in
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          };
          lib = pkgs.lib;
          toolchain = with pkgs.fenix;
            combine [
              (complete.withComponents [
                "cargo"
                "clippy"
                "rust-src"
                "rustc"
              ])
              targets.x86_64-unknown-linux-musl.latest.rust-std
              targets.x86_64-pc-windows-gnu.latest.rust-std
            ];
          # Make naersk aware of the tool chain which is to be used.
          naersk-lib = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
          buildPackage = target: { nativeBuildInputs ? [ ], ... }@args:
            naersk-lib.buildPackage (
              {
                inherit name version;
                src = ./.;
                doCheck = false; # a test or two that I left in there are *not* unit tests lol
                strictDeps = true;
              }
              // (lib.optionalAttrs (target != system) {
                CARGO_BUILD_TARGET = target;
              })
              // args
              // {
                nativeBuildInputs = [ pkgs.fenix.complete.rustfmt-preview ] ++ nativeBuildInputs;
              }
            );
        in
        rec {
          packages = {
            default = buildPackage system {
              # todo make sure this is less cringe
              nativeBuildInputs = [ pkgs.makeWrapper ];
              postInstall = ''
                wrapProgram $out/bin/${name} \
                  --prefix LD_LIBRARY_PATH : ${with pkgs; lib.makeLibraryPath [
                    vulkan-loader
                    libxkbcommon
                    wayland

                    # not sure those are exactly what's needed on X11
                    xorg.libX11
                    xorg.libXcursor
                    xorg.libXi
                    xorg.libXrandr
                  ]}
              '';
            };
            x86_64-unknown-linux-musl = buildPackage "x86_64-unknown-linux-musl" {
              nativeBuildInputs = with pkgs; [ pkgsStatic.stdenv.cc ];
              CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS = "-C target-feature=+crt-static";
            };
            x86_64-pc-windows-gnu = buildPackage "x86_64-pc-windows-gnu" {

              # we can run tests with wine ig, cool
              # this needs some fixing tho
              # doCheck = system == "x86_64-linux";
              # nativeBuildInputs = lib.optional doCheck pkgs.wineWowPackages.stable;
              # CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
              #   # Without this, wine will error out when attempting to create the
              #   # prefix in the build's homeless shelter.
              #   export WINEPREFIX="$(mktemp -d)"
              #   exec wine64 $@
              # '';

              depsBuildBuild = with pkgs.pkgsCross.mingwW64; [
                stdenv.cc
                windows.pthreads
              ];
            };
          };

          apps.default = {
            type = "app";
            program = "${packages.default}/bin/${name}";
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues packages;
            nativeBuildInputs = with pkgs; [
              rust-analyzer-nightly
              cargo-udeps
              cargo-nextest

              wineWowPackages.staging
            ];

            RUST_BACKTRACE = "full";
            RUST_LOG = "info,wgpu_core=warn,wgpu_hal=warn,zbus=warn,noita_utility_box=trace";
          };
        }
      ) // {
      # huh?. we doing hydra now?
      hydraJobs = {
        inherit (self.packages) x86_64-linux;
      };
    };
}

