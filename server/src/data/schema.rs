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
    pub detected_at_utc: chrono::DateTime<chrono::Utc>,
    pub installed_at_utc: chrono::DateTime<chrono::Utc>,

    pub buildid_old: u32,
    pub buildid_new: u32,
}

pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";

pub fn read_sqlite_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
    connection.query_row(READ_SQLITE_VERSION, [], |row| row.get(0))
}
