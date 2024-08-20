{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation rec {
  name = "ori";
  
  buildInputs = [
    pkgs.libGL

    pkgs.libxkbcommon
    pkgs.xorg.libxcb
    pkgs.wayland
  ];

  nativeBuildInputs = [
    pkgs.pkg-config
  ];

  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
}
