{
  description = "Noita Utility Box - a collection of memory-reading utilities for the game Noita";

  nixConfig = {
    extra-substituters = [ "https://necauqua.cachix.org" ];
    extra-trusted-public-keys = [ "necauqua.cachix.org-1:XG5McOG0XwQ9kayUuEiEn0cPoLAMvc2TVs3fXqv/7Uc=" ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # I usually go with oxalica + nixpkkgs rust builder,
    # but fenix + naersk seem to be more convenient for the
    # windows cross-compilation
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
          toolchain = with pkgs.fenix;
            combine [
              (complete.withComponents [
                "cargo"
                "clippy"
                "rust-src"
                "rustc"
              ])
              targets.x86_64-pc-windows-gnu.latest.rust-std
            ];
          naersk-lib = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };

          runtimeDeps = with pkgs; lib.makeLibraryPath [
            vulkan-loader

            # It's annoying that you need either wayland or the xorg stuff,
            # but never both - idk how to make this better, and having to have
            # an LD_LIBRARY_PATH wrapper thing is cringe on its own
            wayland
            libxkbcommon

            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];

          buildPackage = attrs: naersk-lib.buildPackage (
            {
              inherit name version;
              src = ./.;
              strictDeps = true;

              # a test or two that I left in there are *not* unit tests lol
              # todo fix this
              doCheck = false;
            } // attrs
          );
        in
        rec {
          packages = {
            default = buildPackage {
              nativeBuildInputs = [ pkgs.makeWrapper ];
              postInstall = ''
                wrapProgram $out/bin/${name} --prefix LD_LIBRARY_PATH : ${runtimeDeps}
              '';
            };
            windows = buildPackage {
              depsBuildBuild = with pkgs.pkgsCross.mingwW64; [
                stdenv.cc
                windows.pthreads
              ];

              CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

              # # This can run the tests with wine once we have/fix them
              #
              # nativeBuildInputs = [ pkgs.wineWowPackages.staging ];
              #
              # # run the given .exe with wine in a temp prefix
              # CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeShellScript "wine-wrapper" ''
              #   export WINEPREFIX="$(mktemp -dt ${name}-wineprefix-XXXXXX)"
              #   exec wine64 $@
              # '';
            };
          };

          apps.default = {
            type = "app";
            program = "${packages.default}/bin/${name}";
          };

          # This is for testing only lul
          # `nix run .#windows-with-wine` (or github:necauqua/noita-utility-box#windows-with-wine if not in the repo)
          # Didn't find a way to run it in the Noita proton prefix in a way that allows sysinfo to find the noita process yet
          apps.windows-with-wine =
            {
              type = "app";
              program =
                let
                  script = with pkgs; writeShellScript "${name}-with-wine" ''
                    export WINEPREFIX="$(mktemp -dt ${name}-wineprefix-XXXXXX)"
                    trap "rm -rf \"$WINEPREFIX\"" EXIT
                    ${wineWowPackages.staging}/bin/wine64 ${packages.windows}/bin/${name}.exe
                  '';
                in
                "${script}";
            };

          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues packages;
            nativeBuildInputs = with pkgs; [
              rust-analyzer-nightly
              cargo-udeps
              cargo-nextest
            ];

            LD_LIBRARY_PATH = runtimeDeps;
            RUST_BACKTRACE = "full";
            RUST_LOG = "info,wgpu_core=warn,wgpu_hal=warn,zbus=warn,noita_utility_box=trace";
          };
        }
      );
}

