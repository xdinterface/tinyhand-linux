self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.tinyhand;
  toml = pkgs.formats.toml {};
in {
  options.programs.tinyhand = {
    enable = lib.mkEnableOption "TinyHand, a tiny hand on a stick that points at things on your screen";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "The TinyHand package.";
    };

    settings = lib.mkOption {
      type = toml.type;
      default = {};
      example = {
        pointer = "robot";
        size = 0.15;
        ink-fade = 6;
      };
      description = ''
        Written to ~/.config/tinyhand/config.toml. Leave empty to keep the file
        writable, so the toolbar and the settings window can save changes.
        A non-empty value makes the file read-only: changes then last until
        TinyHand quits.
      '';
    };

    systemd.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Start TinyHand with the graphical session.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."tinyhand/config.toml" = lib.mkIf (cfg.settings != {}) {
      source = toml.generate "tinyhand-config.toml" cfg.settings;
    };

    systemd.user.services.tinyhand = lib.mkIf cfg.systemd.enable {
      Unit = {
        Description = "TinyHand pointer";
        PartOf = ["graphical-session.target"];
        After = ["graphical-session.target"];
      };
      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 2;
      };
      Install.WantedBy = ["graphical-session.target"];
    };
  };
}
