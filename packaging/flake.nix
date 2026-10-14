{
  description = "hermetic: a static site generator for Markdown notes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          fs = pkgs.lib.fileset;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "hermetic";
            version = (pkgs.lib.importTOML ../Cargo.toml).package.version;
            # Only what the crate builds and tests from, so editing the wiki or the
            # TypeScript sources does not rebuild it (the browser script is committed).
            src = fs.toSource {
              root = ../.;
              fileset = fs.unions [
                ../Cargo.toml
                ../Cargo.lock
                ../src
                ../static
                ../tests
              ];
            };
            cargoLock.lockFile = ../Cargo.lock;
            # The build script reads the commit from git, which the sandbox lacks.
            HERMETIC_COMMIT = self.shortRev or "";
            meta.mainProgram = "hermetic";
          };
        }
      );

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [
            clippy
            rustfmt
            rust-analyzer
            bun
          ];
        };
      });
    };
}
