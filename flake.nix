{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        craneLib = crane.mkLib pkgs;

        buildInputs = with pkgs; [
          libxkbcommon
          wayland
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        libraryPath = pkgs.lib.makeLibraryPath (with pkgs; [
          vulkan-loader
          wayland
        ]);

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
          inherit nativeBuildInputs buildInputs;
          LD_LIBRARY_PATH = libraryPath;
        };
      }
    );
}
