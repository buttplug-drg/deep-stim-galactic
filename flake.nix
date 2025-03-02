{
    inputs = {
        flake-utils.url = "github:numtide/flake-utils";
        nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    };

    outputs = { self, flake-utils, nixpkgs }:
        flake-utils.lib.eachDefaultSystem (system:
            let
                pkgs = nixpkgs.legacyPackages.${system};
            in {
                devShells.default = pkgs.mkShell {
                    packages = with pkgs; [
                        cargo
                        rustup
                        cargo-cross
                    ];
                    shellHook = ''
                        rustup default stable
                    '';
                };
            }
        );
}
