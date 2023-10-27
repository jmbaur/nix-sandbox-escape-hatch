{ args, requiredSystemFeatures ? [ ], name ? "escape-hatch-derivation" }:
builtins.derivation {
  inherit name requiredSystemFeatures;
  system = builtins.currentSystem;
  builder = "/bin/hatch";
  args = [ "client" ] ++ args;
  NIX_SANDBOX_ESCAPE_HATCH_PATH = "/run/hatch.sock";
}
