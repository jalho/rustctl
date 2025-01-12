# TODO

- Remove _strace_ wraps from everywhere. It was nice to explore it, but for now
  it's just unnecessary complexity.

- Add support for installing custom Carbon plugins (_rustctl-integration_,
  implement at `github.com/jalho/rds-plugins`).

- Get all configuration from a local SQLite file instead of
  `/etc/rustctl/config.toml`: Should facilitate integrating configuration
  management into a web service.
