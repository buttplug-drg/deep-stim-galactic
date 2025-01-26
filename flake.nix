{
    description = "A basic flake with a shell";
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
        systems.url = "github:nix-systems/default";
        flake-utils = {
            url = "github:numtide/flake-utils";
            inputs.systems.follows = "systems";
        };
    };

    # based on:
    # https://github.com/jraygauthier/jrg-rust-cross-experiment/blob/master/simple-static-rustup-target-windows/shell.nix

    outputs = { nixpkgs, flake-utils, ... }:
        flake-utils.lib.eachDefaultSystem (system:
            let
                pkgs = nixpkgs.legacyPackages.${system};
                pkgs-cross-mingw = import pkgs.path {
                    inherit system;
                    crossSystem = "x86_64-w64-mingw32";
                };

                mingw_w64_cc = pkgs-cross-mingw.stdenv.cc;
                mingw_w64 = pkgs-cross-mingw.windows.mingw_w64;
                mingw_w64_pthreads_w_static = pkgs-cross-mingw.windows.mingw_w64_pthreads;

                rustBuildTargetTriple = "x86_64-pc-windows-gnu";
                rustBuildHostTriple = "x86_64-unknown-linux-gnu";

                rustupHome = toString ./.rustup;
                cargoHome = toString ./.cargo;
                cargoBuildTarget = rustBuildTargetTriple;
                cargoTargetWindowsRunner = "${pkgs.wine}/bin/wine64";
                rustupToolchain = "stable-x86_64-unknown-linux-gnu";
            in {
                devShells.default = pkgs.mkShell {
                    packages = with pkgs; [
                        rustup
                        mingw_w64_cc
                        wine
                    ];
                    RUSTUP_HOME = rustupHome;
                    CARGO_HOME = cargoHome;
                    CARGO_BUILD_TARGET = cargoBuildTarget;
                    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = cargoTargetWindowsRunner;

                    RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
                        mingw_w64
                        mingw_w64_pthreads_w_static
                    ]);
                    shellHook = ''
                        export PATH=$PATH:${cargoHome}/bin
                        export PATH=$PATH:${rustupHome}/toolchains/${rustupToolchain}-${rustBuildHostTriple}/bin/

                        rustup target add "${rustBuildTargetTriple}"
                        '';
                    /* shellHook = ''
                        export PATH=$PATH:${cargoHome}/bin

                        rustup target add "${rustBuildTargetTriple}"
                        ''; */
                };
            }
        );
}
