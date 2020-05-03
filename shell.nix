let
  sources = import nix/sources.nix {};
  nixpkgs = import sources.nixpkgs {};
  unstable = import sources.nixpkgs-unstable {};

in { pkgs ? nixpkgs, ghc ? nixpkgs.ghc }:

with pkgs;

unstable.haskell.lib.buildStackProject {
  name = "thulani";

  inherit ghc;

  buildInputs = [
    haskellPackages.hpack
  ];
}
