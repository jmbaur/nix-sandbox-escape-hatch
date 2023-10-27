{ name
, args
, requiredSystemFeatures ? [ ]
, escapeHatchName
, system ? builtins.currentSystem
}:
builtins.derivation {
  inherit name system requiredSystemFeatures;
  builder = "/bin/hatch-door";
  args = [ "client" ] ++ args;
  NIX_SANDBOX_ESCAPE_HATCH_PATH = "/run/${escapeHatchName}-hatch-door.sock";
}
