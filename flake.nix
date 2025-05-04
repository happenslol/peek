{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};

        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-LymSUIHsnE+VhVMMlGedMs1NcnzJYcn4zEg5Ob+cJ7k=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        buildInputs = with pkgs; [
          libxkbcommon
          wayland
          clang
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        libraryPath = pkgs.lib.makeLibraryPath (with pkgs; [
          libxkbcommon
          vulkan-loader
          wayland
        ]);

        packages = with pkgs; [mold];

        commonArgs = {
          inherit buildInputs nativeBuildInputs;
          src = craneLib.cleanCargoSource ./.;
        };
      in {
        packages.default = craneLib.buildPackage (commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
            postInstall = ''wrapProgram "$out/bin/peek" --prefix LD_LIBRARY_PATH : "${libraryPath}"'';
          });

        devShells.default = pkgs.mkShell {
          inherit packages nativeBuildInputs buildInputs;
          LD_LIBRARY_PATH = libraryPath;
        };
      }
    );
}
