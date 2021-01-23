{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-20.09";
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };

    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  description = "thulani discord bot";

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  } @ inputs: (flake-utils.lib.eachDefaultSystem (system:
    let
      barepkgs = import nixpkgs {
        inherit system;
      };

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

            naerskRust = rust;
          })
        ];
      };

      naersk = (inputs.naersk.lib."${system}".override {
        inherit (pkgs) cargo rustc;
      });

      deps = with pkgs; [
        openssl
        pkgconfig
        libopus
        postgresql
      ];

      pkg = naersk.buildPackage {
        pname = "thulani";
        version = self.rev or "dirty";

        src = pkgs.lib.cleanSource ./.;

        buildInputs = deps;
        remapPathPrefix = true;
      };

    in {
      devShell = pkgs.mkShell {
        buildInputs = (with pkgs; [
          cargo
          rustc

          barepkgs.rustracer
          # (rustracer.overrideAttrs (old: {
            # dontCheck = true;
          # }))
        ]) ++ deps;

        RUST_SRC_PATH = "${pkgs.naerskRust}/lib/rustlib/src/rust";
      };

      defaultPackage = pkg;

      defaultApp = {
        type = "app";
        program = "${pkg}/bin/thulani";
      };
    })
  );
}
