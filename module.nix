{ config, lib, pkgs, ... }:
let
  cfg = config.services.nix-sandbox-escape-hatch;

  builderModule = { name, config, ... }: with lib; {
    options = {
      socketPath = mkOption {
        type = types.path;
        default = "/run/${name}-hatch-door.sock";
        description = mdDoc "Unix domain socket path of the escape hatch for this builder.";
      };

      builder = mkOption {
        type = types.path;
        description = mdDoc ''
          The executable that will be ran when a new client requests a build.
          Client arguments will be sent as-is to this program. These arguments
          should be used carefully and validated as untrusted user input. Anyone
          with access to running nix builds on this machine will be able to send
          arguments to this builder!
        '';
      };

      extraServiceConfig = mkOption {
        type = types.attrs;
        default = { };
        description = mdDoc ''
          Extra config that will be placed on the systemd service's
          `serviceConfig` attrset. This can be used for hardening of the
          builder.
        '';
      };
    };
  };

  makeSocket = name: { socketPath, ... }: {
    name = "${name}-hatch-door";
    value = {
      unitConfig = {
        Description = "${name} nix sandbox escape hatch";
        Documentation = [ "https://github.com/jmbaur/nix-sandbox-escape-hatch" ];
      };
      socketConfig = {
        ListenStream = toString socketPath;
        Accept = true;
        SocketMode = "0060";
        SocketGroup = "nixbld";
      };
      wantedBy = [ "sockets.target" ];
    };
  };

  makeService = name: { builder, extraServiceConfig, ... }: {
    name = "${name}-hatch-door@";
    value = {
      unitConfig = {
        Description = "${name} nix sandbox escape hatch";
        Documentation = [ "https://github.com/jmbaur/nix-sandbox-escape-hatch" ];
      };
      serviceConfig = extraServiceConfig // {
        RuntimeDirectory = "hatch-door";
        ExecStart = "${lib.getExe cfg.package} server ${builder}";
      };
    };
  };
in
{
  options.services.nix-sandbox-escape-hatch = with lib; {
    enable = mkEnableOption "nix sandbox escape hatch";

    package = mkPackageOptionMD pkgs "nix-sandbox-escape-hatch" { };

    builders = mkOption {
      type = types.attrsOf (types.submodule builderModule);
      default = { };
      description = mdDoc "Definition of escape-hatch builders.";
    };
  };

  config = lib.mkIf cfg.enable {
    nix.settings.sandbox-paths = [ "/bin/hatch-door=${lib.getExe cfg.package}" ]
      ++ lib.mapAttrsToList (_: { socketPath, ... }: socketPath) cfg.builders;

    systemd.sockets = lib.mapAttrs' makeSocket cfg.builders;
    systemd.services = lib.mapAttrs' makeService cfg.builders;
  };
}

