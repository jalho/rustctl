# TODO

- Leave installation of SteamCMD itself out of scope of the project: The current
  installation by just downloading and extracting some _tar_ archive is not
  enough at least on the Ubuntu 24 that is available for WSL! Note that SteamCMD
  exists as a package in _apt_ at least: `steamcmd`.

- Remove _strace_ wraps from everywhere. It was nice to explore it, but for now
  it's just unnecessary complexity.

- Add support for installing custom Carbon plugins (_rustctl-integration_,
  implement at `github.com/jalho/rds-plugins`).

- Get all configuration from a local SQLite file instead of
  `/etc/rustctl/config.toml`: Should facilitate integrating configuration
  management into a web service.
