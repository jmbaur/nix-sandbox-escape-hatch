# An escape hatch for the nix sandbox

An escape hatch to the nix sandbox without turning off the sandbox! This
program works by taking advantage of nix's `sandbox-paths` setting
(nix.conf(5)). A Unix domain socket is created and passed into the sandbox,
which is used by a client program to send build arguments to a pre-defined
builder outside the sandbox. Systemd accepts traffic on the socket from the
client and triggers a oneshot service that accepts all build arguments,
performs the build, and sends all build artifacts back to the client over the
same socket. The build artifacts are unpacked by the client into the `$out`
path, and everything after that is normal nix!

# Example usage

## Server setup

1. Import this repo's flake output `nixosModules.default` module in your
   configuration.
1. Add some configuration like below. Note that the builder expects to create
   some output in `$out`, just like a derivation output. See below for why we
   setup system features in our nix.conf.
```nix
{ pkgs, ...}: {
    nix.settings.system-features = [ "my-special-feature" ];
    services.nix-sandbox-escape-hatch = {
        enable = true;
        builders.test.builder = pkgs.writeShellScript "test-escape-hatch.bash" ''
            mkdir -p $out
            touch $out/$1
        '';
    };
}
```
1. Activate this new configuration.

## Client setup

1. Import this repo's flake output `overlays.default` overlay into
   your setup.
1. Optionally use the helper `makeEscapeHatchRunner` to create a new function
   for your builder that you have pre-configured (see above). Note that by
   nature of escaping the nix sandbox we reduce the likelyhood of any machine
   with nix installed from being able to successfully build our derivation. Use
   of `requiredSystemFeatures` is _highly_ recommended as a way of filtering
   which builder the derivation can be built on.
```nix
let
    runMyEscapeHatch = makeEscapeHatchRunner "test" {
        requiredSystemFeatures = [ "my-special-feature" ];
    };
in
runMyEscapeHatch "test-foo" [ "foo" ]
```
1. Write
1. `nix build` and see that you now have a file at `./result/foo`!
