{
  description = "TinyHand for Linux: a tiny hand on a stick that points at things on your screen";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux"];
    forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    packages = forAll (pkgs: {
      default = pkgs.callPackage ./nix/package.nix {};
    });

    homeModules.default = import ./nix/hm-module.nix self;
    homeManagerModules.default = self.homeModules.default;

    devShells = forAll (pkgs: {
      default = pkgs.mkShell {
        inputsFrom = [self.packages.${pkgs.stdenv.hostPlatform.system}.default];
        nativeBuildInputs = with pkgs; [clippy rustfmt rust-analyzer];
      };
    });

    formatter = forAll (pkgs: pkgs.alejandra);
  };
}
