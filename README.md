# `rustctl`

Tooling for running a Rust (the game) server and an integrated web service on Linux.

### Intended usage

First, initialize a shared, confidential configuration in the filesystem:

```
rustctl config init
```

The configuration file will be at `/etc/rustctl/config.toml` by default.

Then, enable three independent services using `systemd`. All of them will refer to the shared
configuration file.

1. `rustctl web start`

   The game and the web server are integrated over a Unix domain socket. Since the game can only
   write to the socket once its reader (the web server) has initialized it, the web server must be
   started before the game. This command will start the web server.

2. `rustctl game start`

   This will install or update [SteamCMD][steamcmd-homepage] (game installer),
   [RustDedicated][rustdedicated-homepage] (the game) and [Carbon][carbon-homepage] (a modding
   framework for the game) and then launch the game server.

3. `rustctl health start`

   This will monitor the game server's health and restart it when necessary.

[carbon-homepage]: https://carbonmod.gg/
[rustdedicated-homepage]: https://steamdb.info/app/258550
[steamcmd-homepage]: https://developer.valvesoftware.com/wiki/SteamCMD
