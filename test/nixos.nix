{ module, nixosTest }:
nixosTest {
  name = "nix-sandbox-escape-hatch";

  nodes.machine = { pkgs, ... }: {
    imports = [ module ];

    services.nix-sandbox-escape-hatch = {
      enable = true;
      builder = pkgs.writeShellScript "builder.bash" ''
        mkdir -p $out
        touch $out/$1
      '';
    };
  };

  testScript = ''
    machine.wait_for_unit("sockets.target")
    machine.copy_from_host("${../builder.nix}", "/tmp/buildme.nix")
    machine.succeed("nix-build --arg args '[\"foobar\"]' --out-link /tmp/result /tmp/buildme.nix")
    machine.succeed("test -f /tmp/result/foobar")
  '';
}
