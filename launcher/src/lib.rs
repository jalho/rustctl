pub const UNIX_DOMAIN_SOCKET: &'static str = "/tmp/rustctl.sock";

pub struct GameServerConfig {}

impl Default for GameServerConfig {
    fn default() -> Self {
        Self {}
    }
}

pub fn launch_game_server(_config: &GameServerConfig) -> std::process::ExitCode {
    /*
     * TODO(LLM):
     *
     *   Assume `steamcmd` installed: check. If not, exit with error code.
     *
     *   Install or update `RustDedicated` using `steamcmd`.
     *
     *   Install or update _Carbon_ from GitHub.
     *
     *   Launch RustDedicated and run it to termination: exit with whatever code
     *   the game server exited with.
     *
     *   Launching: load Carbon modding framework and our custom plugin
     *   in it. The plugin should write data about the game's state to
     *   `UNIX_DOMAIN_SOCKET`, i.e. the path needs to be substituted into the
     *   plugin source at load time.
     *
     *   Out of scope: all commands issued via RCON WebSocket client to the
     *   running game server. The managing web server shall take care of that.
     *
     *   Installing Carbon from GitHub:
     *
     *   ```
     *   $ wget https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz
     *
     *   $ ls
     *   Carbon.Linux.Minimal.tar.gz
     *
     *   $ mkdir temp
     *
     *   $ tar -xzf Carbon.Linux.Minimal.tar.gz -C temp/
     *
     *   $ tree -L 2 temp/
     *   temp/
     *   ├── carbon
     *   │   ├── configs
     *   │   ├── data
     *   │   ├── extensions
     *   │   ├── managed
     *   │   ├── native
     *   │   ├── plugins
     *   │   └── tools
     *   ├── carbon.sh
     *   ├── Carbon.targets
     *   └── libdoorstop.so
     *   ```
     *
     *   The `wget`, `tar`, etc. commands above are for reference only. Use
     *   mature and popular Rust libraries in actual implementation: `reqwest`
     *   for HTTP and whatever does the `tar -xzf` trick.
     *
     *   Running the game server: don't use a log file: let `systemd` take care
     *   of collecting timestamped logs.
     *
     *   Describe these steps approximately in ASD-STE100 in the doc comment of
     *   fn launch_game_server, i.e. using the `/// notation`. Don't rephrase
     *   the implementation, just the main responsibilities of the fn.
     */
    std::process::ExitCode::from(44)
}
