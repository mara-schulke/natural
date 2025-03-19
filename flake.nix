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

              python311
              #python3Packages.numpy
              #python3Packages.sentencepiece
              #python3Packages.torch
              #python3Packages.safetensors
              #python3Packages.transformers
              #python3Packages.tokenizers
              #python3Packages.accelerate
              #python3Packages.tensorflow-deps
              #python3Packages.pytorch
              #python3Packages.torchvision
              #python3Packages.torchaudio
              #python3Packages.tensorflowWithoutCuda
              #python3Packages.pip
              #python3Packages.virtualenv
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              #python3Packages.tensorflow-macos
              #python3Packages.tensorflow-metal
              libiconv
              darwin.apple_sdk.frameworks.SystemConfiguration
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.Foundation
              darwin.apple_sdk.frameworks.Metal
              darwin.ICU.dev
              darwin.ICU
              # darwin.xcode_16_2
            ];

          shellHook = ''
            if [ ! -d ".venv" ]; then
              python -m venv .venv
              source .venv/bin/activate
              pip install -r requirements.txt
            else
              source .venv/bin/activate
            fi

            export RUSTFLAGS="-Clink-args=-Wl,-undefined,dynamic_lookup";
            export PKG_CONFIG_PATH="${pkgs.icu}/lib/pkgconfig";
            export LDFLAGS="-L${pkgs.icu}/lib";
            export CPPFLAGS="-I${pkgs.icu}/include";

            # by default, run against pg17
            PG_VERSION=pg17

            pgrx-install() {
              cargo pgrx install -c $HOME/.pgrx/17.*/pgrx-install/bin/pg_config
            }
          '';
        };
      }
    );
}
