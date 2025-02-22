pub struct NotInstalled {}
impl NotInstalled {
    pub fn install_latest_version_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated, std::process::ExitCode> {
        todo!();
    }
}

pub struct InstalledNotRunningNotUpdated {}
impl InstalledNotRunningNotUpdated {
    pub fn update_existing_installation_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated, std::process::ExitCode> {
        todo!();
    }
}

pub struct InstalledNotRunningUpdated {}
impl InstalledNotRunningUpdated {
    pub fn spawn_game_server_process(self) -> Result<RunningNotHealthy, std::process::ExitCode> {
        todo!();
    }
}

pub struct RunningNotHealthy {}
impl RunningNotHealthy {
    pub fn healthcheck_timeout(self) -> Result<RunningHealthy, std::process::ExitCode> {
        todo!();
    }
}

pub struct RunningHealthy {}
impl RunningHealthy {
    pub fn wait(self) -> std::process::ExitCode {
        return std::process::ExitCode::FAILURE;
    }
}
