{ config, lib, pkgs, ... }:
let
  cfg = config.services.nix-sandbox-escape-hatch;
in
{
  options.services.nix-sandbox-escape-hatch = with lib; {
    enable = mkEnableOption "nix sandbox escape hatch";

    package = mkPackageOptionMD pkgs "nix-sandbox-escape-hatch" { };

    socketPath = mkOption {
      type = types.path;
      default = "/run/hatch.sock";
    };

    builder = mkOption {
      type = types.path;
    };
  };

  config = lib.mkIf cfg.enable {
    nix.settings.sandbox-paths = [
      "${cfg.socketPath}"
      "/bin/hatch=${lib.getExe cfg.package}"
    ];

    systemd.sockets.hatch = {
      unitConfig.Description = "nix-sandbox-escape-hatch";
      socketConfig = {
        ListenStream = toString cfg.socketPath;
        Accept = true;
        SocketMode = "0660";
        SocketGroup = "nixbld";
      };
      wantedBy = [ "sockets.target" ];
    };

    systemd.services."hatch@" = {
      unitConfig.Description = "nix-sandbox-escape-hatch";
      serviceConfig = {
        RuntimeDirectory = "hatch";
        ExecStart = "${lib.getExe cfg.package} server ${cfg.builder}";
      };
    };
  };
}
