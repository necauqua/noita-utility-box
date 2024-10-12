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

              NIX_REV = self.rev or "dirty";
            } // attrs
          );
        in
        rec {
          packages = {
            default = buildPackage {
              nativeBuildInputs = with pkgs; [ makeWrapper copyDesktopItems ];
              postInstall = ''
                wrapProgram $out/bin/${name} --prefix LD_LIBRARY_PATH : ${runtimeDeps}
                mkdir -p $out/share/icons/hicolor/256x256/apps
                cp ${./res/icon.png} $out/share/icons/hicolor/256x256/apps/${name}.png
              '';
              desktopItems = [
                (pkgs.makeDesktopItem {
                  name = "Noita Utility Box";
                  exec = "${name}";
                  icon = "${name}";
                  desktopName = "Noita Utility Box";
                  comment = description;
                  categories = [ "System" "Utility" "Debugger" "Amusement" ];
                })
              ];
            };
            # my bad, "unpatches your nix executable"
            # this still depends on like glibc 2.39 so gl running this on not newest ubuntu
            linux = pkgs.stdenv.mkDerivation {
              inherit name version;
              src = packages.default;
              phases = [ "installPhase" ];
              installPhase = ''
                cp -r $src $out
                chmod -R +w $out
                mv $out/bin/{.${name}-wrapped,${name}}
                ${pkgs.patchelf}/bin/patchelf $out/bin/${name} \
                  --set-interpreter "/lib64/ld-linux-x86-64.so.2" \
                  --set-rpath ""
              '';
            };
            # this is made significantly easier by the fact that we don't have
            # any system dependencies other than runtime x11/wayland stuff
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
                EOF
                ${pkgs.dpkg}/bin/dpkg-deb --build package
                mv package.deb $out
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
              pkgs.fenix.default.rustfmt-preview
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

