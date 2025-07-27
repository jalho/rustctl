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
│   ├── tui ............. A terminal app: For development.
│   │
│   └── web ............. A web app: The actual app.
│
├── common .............. Shared libraries.
│
├── mocks
│   │
│   ├── RustDedicated ... Mocks the game server: For development.
│   │
│   └── steamcmd ........ Mocks the game server installer: For development.
│
└── server .............. Backend of the actual app: Manages the game server.
```

## State machine for managing the game server

<img src="./diagrams/rustctl-state-machine.svg">

## Actors in the server

<img src="./diagrams/rustctl-server-actors.svg">

## Cheatsheet

#### Running a light game server

It seems the minimum world size (settable with `+server.worldsize`) is 1000. In
order for a player to be able to spawn on such server, you must issue command
`antihack.terrain_protection 0` via RCON or somehow define a custom spawn point
because otherwise at least using the default seed 1337 players seem to spawn
under terrain.
