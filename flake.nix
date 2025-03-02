{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";

    polar = {
      url = "github:hemisphere-studio/polar";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.crane.follows = "crane";
    };

    utils.follows = "polar/utils";
  };
  outputs =
    {
      self,
      utils,
      nixpkgs,
      polar,
      ...
    }:

    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs =
            with pkgs;
            [
              icu
              icu.dev
              readline.dev
              bison
              zlib
              pkg-config
              flex
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.SystemConfiguration
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.Foundation
              darwin.ICU.dev
              darwin.ICU
            ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.icu}/lib/pkgconfig";
            export LDFLAGS="-L${pkgs.icu}/lib"
            export CPPFLAGS="-I${pkgs.icu}/include"
          '';
        };

        packages.pgpt = polar.lib.buildPgrxExtension {
          inherit system;

          postgresql = pkgs.postgresql_16;
          src = ./.;
        };

        checks.pgpt = self.packages.${system}.pgpt;
      }
    );
}
