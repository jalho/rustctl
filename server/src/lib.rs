#[derive(Clone)]
pub struct PresumedFilesystemHierarchy;

/// Presumed layout of important filesystem paths. Filesystem Hierarchy Standard
/// (FHS) compliance is attempted.
impl PresumedFilesystemHierarchy {
    /// Absolute path to the SteamCMD executable ("game server installer").
    ///
    /// This is expected to be pre-installed system-wide by a package manager.
    const INSTALLER_ABS: &'static str = "/usr/bin/steamcmd";

    /// Absolute path to the `RustDedicated` executable ("game server").
    ///
    /// Contains state information (of the game), therefore `/var/lib/`, and is
    /// managed by `rustctl`, therefore `/var/lib/rustctl/`.
    ///
    /// `RustDedicated` is installed at runtime using SteamCMD.
    ///
    /// Per FHS, executables and state files should presumably be separated. We
    /// can't do that because the game server generates its state files relative
    /// to itself and we cannot control that behavior AFAIK. Our philosophy
    /// is that we consider `RustDedicated` and anything associated with it,
    /// `rustctl`'s application data.
    const GAME_ABS: &'static str = "/var/lib/rustctl/RustDedicated";

    /// Created at runtime as needed.
    const TEMP_DIR_ABS: &'static str = "/tmp/rustctl/";

    /// Created at runtime as needed.
    const SOCKET_ABS: &'static str = "/tmp/rustctl.sock";

    fn temp_dir_abs(&self) -> &std::path::Path {
        std::path::Path::new(Self::TEMP_DIR_ABS)
    }

    fn root_dir_abs(&self) -> &std::path::Path {
        std::path::Path::new(Self::GAME_ABS)
            .parent()
            .expect("game server executable is in a directory")
    }

    fn plugins_dir_abs(&self) -> std::path::PathBuf {
        let mut path = self.game_abs().to_path_buf();
        path.push("carbon/plugins");
        path
    }

    fn socket_abs(&self) -> &std::path::Path {
        std::path::Path::new(Self::SOCKET_ABS)
    }

    fn installer_abs(&self) -> &std::path::Path {
        std::path::Path::new(Self::INSTALLER_ABS)
    }

    fn game_abs(&self) -> &std::path::Path {
        std::path::Path::new(Self::GAME_ABS)
    }

    fn manifest_abs(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_abs().to_path_buf();
        path.push("steamapps/appmanifest_258550.acf");
        path
    }

    fn startup_script_abs(&self) -> std::path::PathBuf {
        let mut path = self.game_abs().to_path_buf();
        path.push("rustctl-run-with-carbon.sh");
        path
    }

    fn carbon_init_script_abs(&self) -> std::path::PathBuf {
        let mut path = self.game_abs().to_path_buf();
        path.push("carbon/tools/environment.sh");
        path
    }

    fn current_game_map_abs(&self) -> std::path::PathBuf {
        let mut path = self.game_abs().to_path_buf();
        path.push("current-game-world-map.png");
        path
    }

    /// Returns the temporary directory path as a UTF-8 string.
    ///
    /// The directory is intended as e.g. temporary storage location for Carbon
    /// Modding Framework installation archive that is downloaded from internet
    /// at runtime.
    pub fn temp_dir_abs_utf8(&self) -> String {
        self.temp_dir_abs().to_string_lossy().to_string()
    }

    /// Returns the game root directory path as a UTF-8 string.
    pub fn root_dir_abs_utf8(&self) -> String {
        self.root_dir_abs().to_string_lossy().to_string()
    }

    /// Returns the Carbon Modding Framework plugins directory path as a UTF-8
    /// string.
    pub fn plugins_dir_abs_utf8(&self) -> String {
        self.plugins_dir_abs().to_string_lossy().to_string()
    }

    /// Returns the Unix domain socket path as a UTF-8 string.
    ///
    /// The socket is intended to be used for inter-process communication
    /// between a Carbon instrumented game server process and `rustctl`.
    pub fn socket_abs_utf8(&self) -> String {
        self.socket_abs().to_string_lossy().to_string()
    }

    /// Returns the SteamCMD executable path as a UTF-8 string.
    ///
    /// This is the game server installer.
    pub fn installer_abs_utf8(&self) -> String {
        self.installer_abs().to_string_lossy().to_string()
    }

    /// Returns the RustDedicated executable path as a UTF-8 string.
    ///
    /// This is the game server.
    pub fn game_abs_utf8(&self) -> String {
        self.game_abs().to_string_lossy().to_string()
    }

    /// Returns the Steam app manifest file path as a UTF-8 string.
    ///
    /// The file contains some metadata of the game server installation.
    pub fn manifest_abs_utf8(&self) -> String {
        self.manifest_abs().to_string_lossy().to_string()
    }

    /// Returns the game server startup script path as a UTF-8 string.
    ///
    /// The script is generated at runtime.
    pub fn startup_script_abs_utf8(&self) -> String {
        self.startup_script_abs().to_string_lossy().to_string()
    }

    /// Returns the Carbon env init script path as a UTF-8 string.
    ///
    /// The script is included in the Carbon installation.
    pub fn carbon_init_script_abs_utf8(&self) -> String {
        self.carbon_init_script_abs().to_string_lossy().to_string()
    }

    pub fn current_game_map_abs_utf8(&self) -> String {
        self.current_game_map_abs().to_string_lossy().to_string()
    }
}
