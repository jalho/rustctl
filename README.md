# `rustctl`

### Intended usage

First, initialize a shared, confidential configuration in the filesystem: `rustctl config init`.
Then, enable three independent services using `systemd` (all of them will refer to the shared
configuration file):

1. `rustctl game start`
2. `rustctl health start`
3. `rustctl web start`

### Manual

```txt
rustctl v0.0.0

SYNOPSIS

      Set up a Rust game server, monitor its health or run a web server
      integrated to the game.

EXAMPLES

    # Write default or show current configuration in `/etc/rustctl/config.toml`:

      rustctl config show
      rustctl config init

    # Start the game server: Install, update, run:

      rustctl game start

    # Monitor the game server's health, restarting it when necessary:

      rustctl health start

    # Start the integrated web server:

      rustctl web start

    # Other stuff you might expect:

      rustctl --help
      rustctl --version
```
