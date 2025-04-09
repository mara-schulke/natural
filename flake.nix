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

        packages = {
          natural = polar.lib.buildPgrxExtension {
            inherit system;
            postgresql = pkgs.postgresql_17;
            src = ./.;
          };
        };

        listToAttrset = list: fn: lib.foldl' lib.attrsets.recursiveUpdate { } (map fn list);

        checks = listToAttrset [ "13" "14" "15" "16" "17" ] (postgresql: {
          "natural-${postgresql}" = polar.lib.buildPgrxExtension {
            inherit system;
            postgresql = pkgs."postgresql_${postgresql}";
            src = ./.;
          };
        });
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
              postgresql
              postgresql.dev
              glib
              gnumake
              clang

              llvmPackages.libclang
              llvmPackages.libclang.lib

              blas
              lapack
              openblas
    
              cudaPackages.cudatoolkit
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

          nativeBuildInputs = [ pkgs.pkg-config ];


            # export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
            # export C_INCLUDE_PATH="${pkgs.lib.makeSearchPathOutput "dev" "include" [ pkgs.stdenv.cc.cc ]}:$C_INCLUDE_PATH"
            # export CPLUS_INCLUDE_PATH="${pkgs.lib.makeSearchPathOutput "dev" "include" [ pkgs.stdenv.cc.cc ]}:$CPLUS_INCLUDE_PATH"
            # export LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ]}:$LIBRARY_PATH"
            # export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ]}:$LD_LIBRARY_PATH"

          shellHook = with pkgs; ''
            export RUSTFLAGS="-Clink-args=-Wl,-undefined,dynamic_lookup";
            export PKG_CONFIG_PATH="${icu}/lib/pkgconfig";
            export LDFLAGS="-L${icu}/lib";
            export CPPFLAGS="-I${icu}/include";
            export CUDA_PATH="${cudaPackages.cudatoolkit}";
            export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib";

            PG_VERSION=pg17
          '';
        };
      }
    );
}
