# Copyright lowRISC contributors.
# Licensed under the Apache License, Version 2.0, see LICENSE for details.
# SPDX-License-Identifier: Apache-2.0
{
  description = "Flake for building aig-manip";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, ... }@inputs: let
    supportedSystems = [ "x86_64-linux" ];
  in {
    packages = nixpkgs.lib.genAttrs supportedSystems (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Use a "nightly" rust toolchain because giputils depends on #![feature(...)], which is not
        # supported by stable releases of Rust.
        toolchain = inputs.fenix.packages.${system}.toolchainOf {
          channel = "nightly";
          date = "2025-07-29";
          sha256 = "sha256-6D2b7glWC3jpbIGCq6Ta59lGCKN9sTexhgixH4Y7Nng=";
        };

        rustPlatform = pkgs.makeRustPlatform {
          inherit (toolchain) rustc cargo;
        };
      in {
        default = rustPlatform.buildRustPackage (finalAttrs: {
          pname = "aig-manip";
          version = "0.1";
          src = self;
          cargoHash = "sha256-hUpAsNVUj4Aovk9/8NdDKAv2Le8hsdNkWUD1csMqgYQ=";
          nativeBuildInputs = [
            pkgs.cmake
            pkgs.clang
          ];
          meta = {
            description = "Tool for manipulating aiger files";
            license = pkgs.lib.licenses.asl20;
            maintainers = [ ];
          };
        });
      });
  };
}
