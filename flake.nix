{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };

    naersk.url = "github:nmattia/naersk";
  };

  description = "thulani discord bot";

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  } @ inputs: (flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;

        overlays = [
          (import inputs.nixpkgs-mozilla)

          (self: super: let
            rust = (super.rustChannelOf {
              channel = "nightly";
              date = "2021-01-01";
              sha256 = "9wp6afVeZqCOEgXxYQiryYeF07kW5IHh3fQaOKF2oRI=";
            }).rust.override {
              extensions = ["rust-src"];
            };
          in {
            cargo = rust;
            rustc = rust;
          })
        ];
      };

      naersk = (inputs.naersk.lib."${system}".override {
        inherit (pkgs) cargo rustc;
      });

      pkg = naersk.buildPackage {
        pname = "thulani";
        version = self.rev or "dirty";

        src = pkgs.lib.cleanSource ./.;

        cargoBuildOptions = old: old ++ [ "--offline" ];

        buildInputs = [
        ];
      };

    in {
      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
        ];
      };

      defaultPackage = pkg;

      defaultApp = {
        type = "app";
        program = "${pkg}/bin/thulani";
      };
    })
  );
}
