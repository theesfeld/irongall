{
  description = "irongall — one theme, one typeface, one size for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in rec {
          irongall = pkgs.rustPlatform.buildRustPackage {
            pname = "irongall";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.fontconfig ];
            meta = {
              description = "One 16-color theme, one typeface, one font size for Linux";
              homepage = "https://github.com/theesfeld/irongall";
              license = pkgs.lib.licenses.mit;
              mainProgram = "irongall";
            };
          };
          default = irongall;
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/irongall";
        };
      });
    };
}
