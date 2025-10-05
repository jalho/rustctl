#[derive(Debug, PartialEq)]
pub struct AppDataSchemaVersion {
    /// The application's version that is idiomatically defined `Cargo.toml`.
    ///
    /// For example: `0.1.0-rc1`.
    pub application_version: String,
}

impl AppDataSchemaVersion {
    pub fn new(value: &str) -> Self {
        Self {
            application_version: value.to_owned(),
        }
    }
}

impl std::fmt::Display for AppDataSchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.application_version)
    }
}

#[derive(Debug)]
pub struct User {
    pub id: String,
    pub created_at_utc: chrono::DateTime<chrono::Utc>,
    pub privileged_at_utc: Option<chrono::DateTime<chrono::Utc>>,
    pub steam_id: u64,
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "user ID {user_id}: Steam ID {steam_id} (created {created_at}{privileged_at})",
            steam_id = self.steam_id,
            user_id = self.id,
            created_at = self.created_at_utc.date_naive(),
            privileged_at = match self.privileged_at_utc {
                Some(instant) => format!(", privileged {instant}", instant = instant.date_naive()),
                None => "".into(),
            }
        )
    }
}

#[derive(Debug)]
pub struct GameParams {
    pub game_params_id: String,
    pub instance_id: String,
    pub valid_starting_from_inclusive_utc: chrono::DateTime<chrono::Utc>,
    pub world_size: u32,
    pub world_seed: u32,
    pub rcon_password: String,
}

impl std::fmt::Display for GameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "params ID {game_params_id}, instance {instance_id}: world size {world_size}, seed {world_seed} (valid from {valid_from})",
            game_params_id = self.game_params_id,
            instance_id = self.instance_id,
            world_size = self.world_size,
            world_seed = self.world_seed,
            valid_from = self.valid_starting_from_inclusive_utc.date_naive()
        )
    }
}

#[derive(Debug, Clone)]
pub struct Wipe {
    pub game_install_or_update_initiated_at_utc: chrono::DateTime<chrono::Utc>,
    pub game_startup_initiated_at_utc: chrono::DateTime<chrono::Utc>,
    pub game_healthy_at_utc: chrono::DateTime<chrono::Utc>,

    pub buildid: u32,

    pub carbon_version: Option<String>,

    pub world_size: u32,
    pub world_seed: u32,
}

#[derive(Debug, Clone)]
pub struct GameUpdate {
    pub install_started_at_utc: chrono::DateTime<chrono::Utc>,
    pub install_completed_at_utc: chrono::DateTime<chrono::Utc>,

    pub buildid_old: u32,
    pub buildid_new: u32,
}

impl GameUpdate {
    pub fn new(
        install_started_at_utc: &chrono::DateTime<chrono::Utc>,
        install_completed_at_utc: &chrono::DateTime<chrono::Utc>,
        buildid_old: &crate::steam::BuildID,
        buildid_new: &crate::steam::BuildID,
    ) -> Self {
        Self {
            install_started_at_utc: install_started_at_utc.to_owned(),
            install_completed_at_utc: install_completed_at_utc.to_owned(),
            buildid_old: buildid_old.into(),
            buildid_new: buildid_new.into(),
        }
    }
}

pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";

pub fn read_sqlite_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
    connection.query_row(READ_SQLITE_VERSION, [], |row| row.get(0))
}
