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
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

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
              gcc13
              gnumake
              autoconf
              clang

              llvmPackages.libclang
              llvmPackages.libclang.lib

              blas
              lapack
              openblas

              cudaPackages.cudatoolkit
              cudaPackages.cudnn
              cudaPackages.cuda_cudart
              cudaPackages.cuda_nvcc
              cudaPackages.cuda_cccl
              cudaPackages.cuda_cudart
              cudaPackages.cuda_cudart.static
              linuxPackages.nvidia_x11
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

          nativeBuildInputs = with pkgs; [
            pkg-config
            cudaPackages.cudatoolkit
            cudaPackages.cuda_cudart
            cudaPackages.cuda_cudart.static
          ];

          shellHook = with pkgs; ''
            export PATH="${pkgs.gcc13}/bin:$PATH"

            export CC=${cudatoolkit.cc}/bin/gcc 
            export CXX=${cudatoolkit.cc}/bin/g++
            export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib";

            export RUSTFLAGS="-Clink-args=-Wl,-undefined,dynamic_lookup -L${cudaPackages.cuda_cudart.static}/lib -L${cudaPackages.libcublas.static}/lib";
            export CMAKE_CUDA_FLAGS="-L${cudaPackages.cuda_cudart.static}/lib -L${cudaPackages.libcublas.static}/lib"

            export PKG_CONFIG_PATH="${icu}/lib/pkgconfig";
            export LDFLAGS="-L${icu}/lib";
            export CPPFLAGS="-I${icu}/include";

            export CUDA_PATH="${cudaPackages.cudatoolkit}";
            #export CUDA_LIBRARY_PATH="${cudaPackages.cudatoolkit}/lib"
            export LD_LIBRARY_PATH=${linuxPackages.nvidia_x11}/lib:$LD_LIBRARY_PATH
            export LIBRARY_PATH=${linuxPackages.nvidia_x11}/lib:${cudaPackages.cudatoolkit}/lib:$LIBRARY_PATH

            PG_VERSION=pg17
          '';
        };
      }
    );
}
