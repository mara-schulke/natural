{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pgrx = {
      url = "github:justinrubek/pgrx/reintroduce-nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    { flake-parts, rust-overlay }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem =
        {
          pkgs,
          system,
          inputs',
          lib,
          ...
        }:
        {
          devShells.default = lib.mkShell {
            buildInputs = with pkgs; [
              icu
              icu.dev
              darwin.ICU.dev
              darwin.ICU
              readline.dev
              bison
              zlib
              pkg-config
              flex
            ];

            shellHook = ''
              export PKG_CONFIG_PATH="${pkgs.icu}/lib/pkgconfig";
              export LDFLAGS="-L${pkgs.icu}/lib"
              export CPPFLAGS="-I${pkgs.icu}/include"
            '';
          };

          packages = {
            pgpt = inputs.pgrx.lib.buildPgrxExtension {
              inherit system;

              postgresql = pkgs.postgresql_16;
              rustToolchain = inputs'.fenix.packages.stable.toolchain;

              src = ./.;

              #src = inputs.nix-filter.lib {
              #root = ./.;
              #include = [
              #"src"
              #"Cargo.toml"
              #"Cargo.lock"
              #"arrays.control"
              #];
              #};
            };
          };
        };
    };
}
