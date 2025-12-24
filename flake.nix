{
  description = "A collection of memory-reading utilities for the game Noita";

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

    build-env = {
      url = "file+file:///dev/null";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, naersk, flake-utils, fenix, build-env }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      inherit (cargoToml.package) name version description;
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
              (stable.withComponents [
                "cargo"
                "clippy"
                "rust-src"
                "rustc"
              ])
              targets.x86_64-pc-windows-gnu.stable.rust-std
            ];
          naersk-lib = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };

          buildPackage = attrs: naersk-lib.buildPackage (
            {
              inherit name version;
              src = ./.;
              strictDeps = true;

              NIX_REV = self.rev or "dirty";
            } // attrs // builtins.fromTOML (builtins.readFile "${build-env}")
          );

          wineWrap = name: cmd: pkgs.writeShellScript "${name}-in-wine" ''
            export WINEPREFIX="$(mktemp -dt ${name}-wineprefix-XXXXXX)"
            trap "rm -rf \"$WINEPREFIX\"" EXIT
            exec ${pkgs.wineWowPackages.staging}/bin/wine ${cmd}
          '';

          dynamicDeps = with pkgs; lib.makeLibraryPath [
            wayland
            libxkbcommon
            vulkan-loader
            libGL
          ];
        in
        rec {
          packages = {
            default = buildPackage {
              nativeBuildInputs = with pkgs; [
                makeWrapper
                copyDesktopItems
                pkg-config
                autoPatchelfHook
              ];

              buildInputs = with pkgs; [
                gcc.cc.lib
                openssl.out

                xorg.libX11
                xorg.libXcursor
                xorg.libXi
                xorg.libXrandr
              ];

              appendRunpaths = dynamicDeps;

              postInstall = ''
                mkdir -p $out/share/icons/hicolor/256x256/apps
                cp ${./res/icon.png} $out/share/icons/hicolor/256x256/apps/${name}.png
              '';

              desktopItems = [
                (pkgs.makeDesktopItem {
                  inherit name;
                  exec = name;
                  icon = name;
                  desktopName = "Noita Utility Box";
                  comment = description;
                  categories = [ "System" "Utility" "Debugger" "Amusement" ];
                })
              ];
            };
            # my bad, "unpatches your nix executable"
            # this still depends on like glibc 2.39 so gl running this on ubuntu that's not the newest
            linux = pkgs.stdenv.mkDerivation {
              inherit name version;
              src = packages.default;
              phases = [ "installPhase" ];
              installPhase = ''
                cp -r $src $out
                chmod -R +w $out
                ${pkgs.patchelf}/bin/patchelf $out/bin/${name} \
                  --set-interpreter "/lib64/ld-linux-x86-64.so.2" \
                  --set-rpath ""
              '';
            };
            # idk lol, build all the things through nix
            deb = pkgs.stdenv.mkDerivation {
              inherit version;
              name = "${name}.deb";
              src = packages.linux;
              phases = [ "installPhase" ];
              installPhase = ''
                mkdir -p package/{usr,DEBIAN}
                cp -r $src/* package/usr
                cat > package/DEBIAN/control <<EOF
                Package: ${name}
                Version: ${version}
                Architecture: amd64
                Maintainer: necauqua <him@necauq.ua>
                Description: ${description}
                Depends: openssl
                EOF
                ${pkgs.dpkg}/bin/dpkg-deb --build package
                mv package.deb $out
              '';
            };

            windows = buildPackage {
              nativeBuildInputs = with pkgs; [
                pkgsCross.mingwW64.stdenv.cc
                imagemagick
              ];
              buildInputs = [
                pkgs.pkgsCross.mingwW64.windows.pthreads
              ];

              CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

              # can run the tests in wine
              CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = wineWrap "${name}-tests" "$@";
            };
          };

          apps.default = {
            type = "app";
            program = "${packages.default}/bin/${name}";
          };

          # This is for testing only lul
          # `nix run .#windows-with-wine` (or github:necauqua/noita-utility-box#windows-with-wine if not in the repo)
          # Didn't find a way to run it in the Noita proton prefix in a way that allows sysinfo to find the noita process yet
          apps.windows-with-wine = {
            type = "app";
            program = "${wineWrap name "${packages.windows}/bin/${name}.exe"}";
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues packages;

            nativeBuildInputs = with pkgs; [
              rust-analyzer-nightly
              pkgs.fenix.default.rustfmt-preview
              cargo-nextest

              # used in CI to make release artifacts
              p7zip
            ];

            # make `cargo run` work
            LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
              openssl
              dynamicDeps
            ];

            RUSTDOCFLAGS = "-D warnings";
            RUST_BACKTRACE = "full";
            RUST_LOG = "info,wgpu_core=warn,wgpu_hal=warn,zbus=warn,noita_utility_box=trace";
          };
        }
      );
}
