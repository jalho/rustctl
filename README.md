Work in progress!

# `rustctl`

Tooling for running a *Rust* (the game) server and an integrated web service
on Linux.

Video demo as of commit `b7a1e5e1` (2025-11-16): [YouTube](https://youtu.be/cil5wLN7vUo).

## Features

Features are listed in the [`./CHANGELOG.md`](./CHANGELOG.md).

## Repository structure

```
.
├── clients
│   │
│   ├── tui ............. Terminal app intended as a dev tool.
│   │
│   └── web ............. Web app intended as an actual deployment.
│
├── server .............. Backend for the clients.
│
└── common .............. Shared between the clients and the server.
```

## State machine managing the game server

An illustration of how the game server is managed, as a state machine:

<img src="./diagrams/rustctl-state-machine.svg">

## Actors in the server

Design diagram in terms of channel primitives:

<img src="./diagrams/rustctl-software-design-in-terms-of-channel-primitives.svg">

## Presumed filesystem hierarchy

Various filesystem entries are presumed pre-existing or created at runtime
at specific paths. Below is a non-exhaustive list of some that might be
interesting. (Not in any meaningful order!)

```
/tmp/
│
├── rustctl.sock ................ Unix domain socket. Created at runtime.
│
└── rustctl/ .................... Temporary directory. Created at runtime as needed.

/usr/bin/
│
└── steamcmd .................... "SteamCMD": Game server installer. Presumed
                                  pre-installed. It's available via e.g. Debian,
                                  Ubuntu and Arch package managers.

/var/lib/rustctl/
│
├── rustctl.db .................. Generated at runtime.
│
├── rustctl.log ................. Generated at runtime.
│
├── carbon
│   │
│   ├── tools
│   │   └── environment.sh ...... Included in "Carbon" installation, which is
│   │                             downloaded from internet at runtime.
│   └── plugins
│       └── rustctl_sock.cs ..... Generated at runtime.
│
├── current-world-map.png ....... Generated at runtime.
│
├── libdoorstop.so .............. Included in the "Carbon" installation.
│
├── rustctl-run-with-carbon.sh .. Generated at runtime.
│
├── RustDedicated ............... The game server. Installed at runtime from
│                                 internet using "SteamCMD". Contains state
│                                 information relative to itself.
│
├── server
│   └── instance0 ............... Generated at runtime (by the game).
│       ├── cfg
│       │   └── users.cfg
│       └── *.sav
│
└── steamapps
    └── appmanifest_258550.acf .. Included in the "RustDedicated" installation.
```

## Development cheatsheet

Make a Debian package (a `.deb` file):

```sh
cargo xtask dist
```

For more tasks, check `--help`:

```sh
cargo xtask --help
```

The idea of implementing the auxiliary tasks of a software project using the
main programming language (as opposed to e.g. Bash scripts) was inspired by:

- [`nob.h` by `tsoding`](https://github.com/tsoding/nob.h)

  > The idea is that you should not need anything but a C compiler to build
  > a C project. No make, no cmake, no shell, no cmd, no PowerShell etc. Only
  > C compiler.

- [`cargo-xtask` by `matklad`](https://github.com/matklad/cargo-xtask)

  > cargo-xtask is way to add free-form automation to a Rust project ...
  > distinguishing features of xtask are: It doesn't require any other binaries
  > besides cargo and rustc ...

Some other commands:

```sh
cargo run -- -i 192.168.0.103 -p 8080 --steam-id-append 76561198135242017
```

Start web app dev server in its root:

```sh
dx serve --platform web --addr 192.168.0.103 --port 8000
```

SQLite cheatsheet:

```sh
sqlite3 /var/lib/rustctl/rustctl.db

sqlite> .tables

sqlite> .schema

sqlite> SELECT * FROM app_data_schema_version;
0.1.0-rc1

sqlite> UPDATE game_params SET world_seed = 1234 WHERE game_params_id = '00000000-0000-0000-0000-000000000000';

sqlite> .quit
```

Bundle web app:

```sh
cd ./clients/web
dx bundle --platform web
cd -
mv ./target/dx/rustctl-web/release/web/public /var/lib/rustctl/web
```

```
├── assets
│   ├── rustctl-web-10c6fdaee3286dde.js
│   ├── rustctl-web-10c6fdaee3286dde.js.br
│   ├── rustctl-web_bg-a5d465d285bbadf8.wasm
│   └── rustctl-web_bg-a5d465d285bbadf8.wasm.br
└── index.html
```

The web server will serve the bundle from `/var/lib/rustctl/web/`.

## Tips

### Using `steamcmd`

The program assumes `steamcmd` to be installed in `/usr/bin/steamcmd` which
is where the Arch Linux's AUR package installs in. Some other package systems
install it elsewhere though: E.g. Debian's APT creates `/usr/games/steamcmd`.
Create a symbolic link from the expected installation path to the actual
installation path if they differ:

```
$ ln -s /usr/games/steamcmd /usr/bin/steamcmd
```
