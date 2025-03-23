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

        inherit (pkgs) lib;

        supportedPostgres = with pkgs; [
          postgresql_13
          postgresql_14
          postgresql_15
          postgresql_16
          postgresql_17
        ];

        listToAttrset = list: fn: lib.foldl' lib.attrsets.recursiveUpdate { } (map fn list);

        packages = listToAttrset supportedPostgres (
          postgresql:
          let
            major = lib.versions.major postgresql.version;
          in
          {
            "natural-${major}" = polar.lib.buildPgrxExtension {
              inherit system postgresql;
              src = ./.;
            };
          }
        );

        checks = listToAttrset supportedPostgres (
          postgresql:
          let
            major = lib.versions.major postgresql.version;
          in
          {
            "natural-${major}" = self.packages.${system}."natural-${major}";
          }
        );
      in
      {
        inherit packages checks;

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
              cmake
              python3
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.SystemConfiguration
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.Foundation
              darwin.apple_sdk.frameworks.Metal
              darwin.ICU.dev
              darwin.ICU
            ];

          shellHook = ''
            export RUSTFLAGS="-Clink-args=-Wl,-undefined,dynamic_lookup";
            export PKG_CONFIG_PATH="${pkgs.icu}/lib/pkgconfig";
            export LDFLAGS="-L${pkgs.icu}/lib";
            export CPPFLAGS="-I${pkgs.icu}/include";

            PG_VERSION=pg17
          '';
        };
      }
    );
}
