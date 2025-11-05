{
  description = "eframe devShell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        rustVersion = "1.90.0";
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShells.default = mkShell rec {
          buildInputs = [
            # Rust
            (rust-bin.stable.${rustVersion}.default.override {
              extensions = [
                "rust-std"
                "rustfmt"
                "rust-src" # for rust-analyzer
                "rust-analyzer"
              ];
              targets = [ "wasm32-unknown-unknown" ];
            })
            trunk

            # misc. libraries
            SDL2.dev
            alsa-lib
            alsa-lib.dev
            clang
            cmake
            gdb
            gtest
            libclang
            libffi.dev
            mold-wrapped
            openssl
            pipewire.dev
            pkg-config
            libudev-zero
            udev
            xz

            # GUI libs
            atk
            ffmpeg.dev
            fontconfig
            gdk-pixbuf
            gtk3
            libGL
            libxkbcommon
            mpv
            pango
            vulkan-loader
            vulkan-tools

            # wayland libraries
            wayland

            # x11 libraries
            libdrm.dev
            mesa-demos
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libXxf86vm
          ];

          packages = [
            just
            rlwrap
            sqlite
            vmtouch
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      });
}
