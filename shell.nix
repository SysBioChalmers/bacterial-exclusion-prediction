{ pkgs ? import <nixpkgs> {}}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    rust-analyzer
    clippy
    tesseract
    pkg-config
    fontconfig
    (python3.withPackages (p : with p; [
      toml scipy numpy matplotlib
    ]))
  ];
}
