{
  description = "Dev environment for Lingua Serpentis";

  inputs = {
    # nixos-unstable; bump with:
    #   nix flake update
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        }));
    in
    {
      devShells = forEachSystem (pkgs:
        let
          # musl targets are meaningless on darwin; only linux hosts get them.
          muslTargets =
            if nixpkgs.lib.hasSuffix "-linux" pkgs.stdenv.hostPlatform.system
            then [ "x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl" ]
            else [ ];
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "clippy" "rustfmt" "rust-analyzer" ];
            targets = muslTargets;
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.wild
              pkgs.qemu
              pkgs.cargo-nextest
              # graphify isn't in nixpkgs: `uv tool install graphifyy`.
              pkgs.python3
              pkgs.uv
            ];

            env.RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          };
        });
    };
}
