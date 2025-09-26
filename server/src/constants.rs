#[rustfmt::skip]
pub mod paths {
  pub const INSTALLER: &str   = "/usr/bin/steamcmd";

  pub const ROOT_DIR: &str    = "/var/lib/rustctl";
  pub const DB: &str          = "/var/lib/rustctl/rustctl.db";
  pub const LOG: &str         = "/var/lib/rustctl/rustctl.log";
  pub const GAME: &str        = "/var/lib/rustctl/RustDedicated";

  pub const MANIFEST: &str    = "/var/lib/rustctl/steamapps/appmanifest_258550.acf";
  pub const STARTUP: &str     = "/var/lib/rustctl/rustctl-run-with-carbon.sh";
  pub const CARBON_INIT: &str = "/var/lib/rustctl/carbon/tools/environment.sh";

  pub const TMP_ARCHIVE: &str = "/tmp/carbon.tgz";

  pub const SOCKET: &str      = "/tmp/rustctl.sock";
  pub const PLUGINS_DIR: &str = "/var/lib/rustctl/carbon/plugins";
  pub const PLUGIN: &str      = "/var/lib/rustctl/carbon/plugins/rustctl_sock.cs";

  pub const GAME_MAP: &str    = "/var/lib/rustctl/current-game-world-map.png";
}

#[rustfmt::skip]
pub mod names {
  pub const GAME_INSTANCE_ID: &str = "instance0";
}
