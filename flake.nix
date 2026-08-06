{
  description = "xrml — the HRML static site generator (eXtensible Rust Markup Language)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      lib = pkgs.lib;
    in
    {
      packages.${system} = let
        xrml = with pkgs; rustPlatform.buildRustPackage {
          pname = "xrml";
          version = "0.1.0";
          src = ./.;

          # Cargo.lock is tracked and has no git dependencies, so the crate
          # tarball is vendored straight from the lockfile. No outputHashes and
          # no hash override needed; a re-lock bump is the whole change.
          cargoLock.lockFile = ./Cargo.lock;

          # The unit tests read templates from the `usi` git submodule, which is
          # not fetched for a source build of this flake. They say nothing about
          # the binary, so skip them rather than fail the build on a submodule
          # this flake does not need.
          doCheck = false;

          meta = with lib; {
            description = "Static-site generator and server for the HRML templating language";
            license = licenses.mit;
            mainProgram = "xrml";
          };
        };
      in {
        inherit xrml;
        default = xrml;
      };

      apps.${system}.default = {
        type = "app";
        program = "${self.packages.${system}.xrml}/bin/xrml";
      };

      devShells.${system}.default = pkgs.mkShell {
        inputsFrom = [ self.packages.${system}.xrml ];
      };
    };
}
