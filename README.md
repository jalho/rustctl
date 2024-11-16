Work in progress!

# `rustctl`

Tooling for running a _Rust_ (the game) server and an integrated web service on Linux.

### Intended usage

First, initialize a shared, confidential configuration in the filesystem:

```
rustctl config init
```

The configuration file will be at `/etc/rustctl/config.toml` by default.

Then, enable and start the three following services using [_systemd_][systemd-website]. All of them
will refer to the shared configuration file.

1. `rustctl web start`

   The game and the web server are integrated over a Unix domain socket. Since the game can only
   write to the socket once its reader (the web server) has initialized it, the web server must be
   started before the game. This command will start the web server.

2. `rustctl game start`

   This will install or update [_SteamCMD_][steamcmd-website] (game installer),
   [_RustDedicated_][rustdedicated-website] (the game) and [_Carbon_][carbon-website] (a modding
   framework for the game) and then launch the game server.

3. `rustctl health start`

   This will monitor the game server's health and restart it when necessary.

### Example configuration

As of commit `TODO`.

```toml
"root_dir"          = "/home/rust/"
"log_level"         = "all" # all | normal
"steamcmd_download" = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz"
"carbon_download"   = "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Release.tar.gz"
```

### Manual tests

| date       | commit  | system                                                     | asserts                                  |
| ---------- | ------- | ---------------------------------------------------------- | ---------------------------------------- |
| 2024-11-12 | e8216f3 | Arch Linux, kernel 6.11.2-arch1-1, strace v6.11, tar v1.35 | Game is fully functional with Carbon.    |
| 2024-10-27 | 7c4ba1d | Arch Linux, kernel 6.11.2-arch1-1, strace v6.11, tar v1.35 | Game server seems healthy based on logs. |
| 2024-10-27 | a8b5ea1 | Debian 12, kernel 6.1.0-26-amd64, strace v6.1, tar v1.34   | Game server seems healthy based on logs. |
| 2024-10-19 | 81e78da | Arch Linux, kernel 6.11.2-arch1-1, strace v6.11, tar v1.35 | Game is fully functional with Carbon.    |
| 2024-10-19 | 81e78da | Debian 12, kernel 6.1.0-26-amd64, strace v6.1, tar v1.34   | Game server seems healthy based on logs. |

### Cheatsheet

#### Running a light server

It seems the minimum world size (settable with `+server.worldsize`) is 1000.
In order for a player to be able to spawn on such server, you must issue command
`antihack.terrain_protection 0` via RCON or somehow define a custom spawn point
because otherwise at least using the default seed 1337 players seem to spawn
under terrain.

[carbon-website]: https://carbonmod.gg
[rustdedicated-website]: https://steamdb.info/app/258550
[steamcmd-website]: https://developer.valvesoftware.com/wiki/SteamCMD
[systemd-website]: https://systemd.io
