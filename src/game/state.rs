pub struct NotInstalled<'res> {
    resources: &'res super::Resources,
}

impl<'res> NotInstalled<'res> {
    pub fn new(resources: &'res super::Resources) -> Self {
        return Self { resources };
    }

    pub fn install_latest_version_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated<'res>, std::process::ExitCode> {
        todo!(
            "install game server using SteamCMD -- see tag state-machine-v1 for an implementation: steamcmd_exec()"
        );
    }
}

pub struct InstalledNotRunningNotUpdated<'res> {
    resources: &'res super::Resources,
}

impl<'res> InstalledNotRunningNotUpdated<'res> {
    pub fn new(resources: &'res super::Resources) -> Self {
        return Self { resources };
    }

    pub fn update_existing_installation_from_remote(
        self,
    ) -> Result<InstalledNotRunningUpdated<'res>, std::process::ExitCode> {
        todo!("check for game server updates and update if necessary using SteamCMD -- see tag state-machine-v1 for an implementation: steamcmd_exec()");
    }
}

pub struct InstalledNotRunningUpdated<'res> {
    resources: &'res super::Resources,
}

impl<'res> InstalledNotRunningUpdated<'res> {
    pub fn new(resources: &'res super::Resources) -> Self {
        return Self { resources };
    }

    pub fn spawn_game_server_process(
        self,
    ) -> Result<RunningNotHealthy<'res>, std::process::ExitCode> {
        todo!(
            "spawn game server process -- see tags 0.1.0, state-machine-v1 for an implementation"
        );
    }
}

pub struct RunningNotHealthy<'res> {
    resources: &'res super::Resources,
}

impl<'res> RunningNotHealthy<'res> {
    pub fn new(resources: &'res super::Resources) -> Self {
        return Self { resources };
    }

    pub fn healthcheck_timeout(self) -> Result<RunningHealthy<'res>, std::process::ExitCode> {
        todo!("wait for game server to become healthy somehow -- wait until timeout to see some promising log from game server?, or terminate program if cannot be determined");
    }
}

pub struct RunningHealthy<'res> {
    resources: &'res super::Resources,
}

impl<'res> RunningHealthy<'res> {
    pub fn new(resources: &'res super::Resources) -> Self {
        return Self { resources };
    }

    pub fn wait(self) -> std::process::ExitCode {
        todo!("run the game server until it terminates -- maybe support for graceful interrupt termination?");
    }
}
