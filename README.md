Work in progress!

# `rustctl`

Tooling for running a _Rust_ (the game) server and an integrated web service
on Linux.

## Features

Features are listed in the [`./CHANGELOG.md`](./CHANGELOG.md).

## Repository structure

```
.
├── clients
│   │
│   ├── tui ............. Terminal app intended as a dev tool.
│   │
│   └── web ............. Web app intended as an actual deployment.
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
└── rustctl.sock ................ Unix domain socket. Created at runtime.

/usr/bin/
│
└── steamcmd .................... "SteamCMD": Game server installer. Presumed
                                  pre-installed. It's available via e.g. Debian,
                                  Ubuntu and Arch package managers.

/home/rust/ ..................... Presumed pre-existing.
│
├── carbon
│   ├── tools
│   │   └── environment.sh ...... Included in "Carbon" installation, which is
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
│                                 internet using "SteamCMD".
│
├── server
│   └── instance0 ............... Generated at runtime (by the game, automatically).
│       ├── cfg
│       │   └── users.cfg
│       └── *.sav
│
└── steamapps
    └── appmanifest_258550.acf .. Included in the "RustDedicated" installation.
```

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
