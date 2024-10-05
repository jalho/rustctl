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

As of commit `2d2f797`. Not complete!

```toml
"rustctl_root_dir" = "/home/rust"

"steamcmd_download_url"           = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz"
"steamcmd_target_file_name_tgz"   = "steamcmd.tgz"
"steamcmd_executable_name"        = "steamcmd.sh"

"steamcmd_installations_dir_name" = "installations"
"game_server_executable_name"     = "RustDedicated"
"game_server_argv" = [
    "-batchmode",
    "+server.identity",
    "instance0",
    "+rcon.port",
    "28016",
    "+rcon.web",
    "1",
    "+rcon.password",
    "Your_Rcon_Password",
]
```

[carbon-website]: https://carbonmod.gg
[rustdedicated-website]: https://steamdb.info/app/258550
[steamcmd-website]: https://developer.valvesoftware.com/wiki/SteamCMD
[systemd-website]: https://systemd.io
