{
  description = "Nix Sandbox Escape Hatch";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: {
    overlays.default = final: prev: {
      nix-sandbox-escape-hatch = prev.callPackage ./default.nix { };
    };
    nixosModules.default = {
      nixpkgs.overlays = [ self.overlays.default ];
      imports = [ ./module.nix ];
    };
    legacyPackages = nixpkgs.lib.genAttrs
      [ "x86_64-linux" "aarch64-linux" ]
      (system: import nixpkgs {
        inherit system;
        overlays = [ self.overlays.default ];
      });
    devShells = nixpkgs.lib.mapAttrs
      (system: pkgs: { default = pkgs.nix-sandbox-escape-hatch; })
      self.legacyPackages;
    packages = nixpkgs.lib.mapAttrs
      (system: pkgs: {
        default = pkgs.nix-sandbox-escape-hatch;
        test = pkgs.callPackage ./test/nixos.nix {
          module = self.nixosModules.default;
        };
      })
      self.legacyPackages;
  };
}
