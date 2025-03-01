{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pgrx = {
      url = "github:justinrubek/pgrx/reintroduce-nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };

    crane.url = "github:ipetkov/crane";
  };
  outputs =
    { utils, nixpkgs, ... }@inputs:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
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

        packages.pgpt = inputs.pgrx.lib.buildPgrxExtension {
          inherit system;

          postgresql = pkgs.postgresql_16;
          rustToolchain = inputs.fenix.packages.${system}.stable.toolchain;

          src = ./pgpt-extension;

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
      }
    );
}
