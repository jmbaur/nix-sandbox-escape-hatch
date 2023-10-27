{ args, name ? "escape-hatch-derivation" }:
builtins.derivation {
  inherit name;
  system = builtins.currentSystem;
  builder = "/bin/hatch";
  args = [ "client" ] ++ args;
  allowSubstitutes = false;
  preferLocalBuild = true;
  NIX_SANDBOX_ESCAPE_HATCH_PATH = "/run/hatch.sock";
}
