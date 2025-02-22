pub struct NotInstalled {}
impl NotInstalled {
    pub fn install_latest_version_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated, std::process::ExitCode> {
        todo!(
            "install game server using SteamCMD -- see tag state-machine-v1 for an implementation: steamcmd_exec()"
        );
    }
}

pub struct InstalledNotRunningNotUpdated {}
impl InstalledNotRunningNotUpdated {
    pub fn update_existing_installation_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated, std::process::ExitCode> {
        todo!("check for game server updates and update if necessary using SteamCMD -- see tag state-machine-v1 for an implementation: steamcmd_exec()");
    }
}

pub struct InstalledNotRunningUpdated {}
impl InstalledNotRunningUpdated {
    pub fn spawn_game_server_process(self) -> Result<RunningNotHealthy, std::process::ExitCode> {
        todo!(
            "spawn game server process -- see tags 0.1.0, state-machine-v1 for an implementation"
        );
    }
}

pub struct RunningNotHealthy {}
impl RunningNotHealthy {
    pub fn healthcheck_timeout(self) -> Result<RunningHealthy, std::process::ExitCode> {
        todo!("wait for game server to become healthy somehow -- wait until timeout to see some promising log from game server?, or terminate program if cannot be determined");
    }
}

pub struct RunningHealthy {}
impl RunningHealthy {
    pub fn wait(self) -> std::process::ExitCode {
        todo!("run the game server until it terminates -- maybe support for graceful interrupt termination?");
    }
}
