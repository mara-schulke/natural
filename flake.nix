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
            "pgpt-${major}" = polar.lib.buildPgrxExtension {
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
            "pgpt-${major}" = self.packages.${system}."pgpt-${major}";
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
              python3
              python3Packages.pybind11
              uv
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

          shellHook = with pkgs; ''
            export RUSTFLAGS="-Clink-args=-Wl,-undefined,dynamic_lookup";
            export PKG_CONFIG_PATH="${icu}/lib/pkgconfig";
            export LDFLAGS="-L${icu}/lib";
            export CPPFLAGS="-I${icu}/include";

            export PYTHONPATH=${python3}/lib/python3.12/site-packages
            export LD_LIBRARY_PATH=${python3}/lib:$LD_LIBRARY_PATH
            export DYLD_LIBRARY_PATH=${python3}/lib:$DYLD_LIBRARY_PATH
            export PYO3_PYTHON=${python3}/bin/python3

            python -c "import sys; print(sys.version)"
            python -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"

            # by default, run against pg17
            PG_VERSION=pg17

            source ./.venv/bin/activate

            exec zsh
          '';
        };
      }
    );
}
