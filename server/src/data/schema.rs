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
    pub instance_id: String,
    pub updated_at_utc: chrono::DateTime<chrono::Utc>,
    pub world_size: u32,
    pub world_seed: u32,
    pub rcon_password: String,
}

impl std::fmt::Display for GameParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "instance {instance_id}: world size {world_size}, seed {world_seed} (updated {updated_at})",
            instance_id = self.instance_id,
            world_size = self.world_size,
            world_seed = self.world_seed,
            updated_at = self.updated_at_utc.date_naive()
        )
    }
}

pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";

pub fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
    connection.query_row(READ_SQLITE_VERSION, [], |row| row.get(0))
}
