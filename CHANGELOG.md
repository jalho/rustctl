# Changelog

## 1.0.0

Initial version. Planned features:

### Added

#### Backend

- Automatic installation and updating of `RustDedicated` (game server) via
  `steamcmd` (installer) at startup.
- Graceful shutdown handling on `SIGINT`, `SIGTERM`, or on any identified
  non-recoverable failure:
  - Save in-game world state
  - Clean up resources: Terminate child processes such as the game server
- Game world map rendering after startup; served over HTTP as a PNG file.
- Player statistics tracking for in-game activities (e.g., collecting resources
  like _wood_).
- WebSocket support for remote clients:
  - Steam-based authentication and identity-based authorization for management
    access.
  - State broadcasts:
    - Server status (_running_, _not running_, etc.)
    - System resource usage (CPU, memory)
    - Game world state (e.g., in-game player positions)
    - Game events (e.g., in-game resource collection)
  - Command reception for:
    - Managing game server (terminate, update, relaunch)
    - Managing game world (_RCON commands_ like banning players, spawning items)

#### Frontend

- Single persistent WebSocket connection to backend with auto-reconnect.
- Visualization of:
  - Game server status
  - In-game world state (e.g., player positions over map)
  - In-game events (e.g., resource collection)
  - System resource usage over time (CPU, memory)
- Controls for:
  - Server management (terminate, update, relaunch)
  - Game world management (_RCON commands_)
