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

#[derive(Debug, Clone)]
pub struct GameWorldSize(u32);

impl GameWorldSize {
    /// The smallest value that I've successfully used. Idk what's the actual
    /// minimum.
    const MIN_INT: u32 = 1000;

    /// The biggest value that I've successfully used or care about. Idk what's
    /// the actual maximum.
    const MAX_INT: u32 = 4500;

    pub const MIN: GameWorldSize = Self::new(Self::MIN_INT);
    pub const MAX: GameWorldSize = Self::new(Self::MAX_INT);

    pub const fn new(value: u32) -> Self {
        match value {
            Self::MIN_INT..Self::MAX_INT => Self(value),
            _ => todo!(),
        }
    }
}

impl std::fmt::Display for GameWorldSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&GameWorldSize> for u32 {
    fn from(value: &GameWorldSize) -> Self {
        value.0
    }
}

#[derive(Debug, Clone)]
pub struct GameParams {
    pub game_params_id: String,
    pub instance_id: String,
    pub valid_starting_from_inclusive_utc: chrono::DateTime<chrono::Utc>,
    pub world_size: GameWorldSize,
    pub world_seed: u32,
    pub rcon_password: String,
}

impl GameParams {
    pub fn new_with_random_seed(
        valid_starting_from_inclusive_utc: &chrono::DateTime<chrono::Utc>,
        world_size: &GameWorldSize,
    ) -> Self {
        /// Docs:
        /// > This number can be any value 0-2147483647
        ///
        /// https://wiki.facepunch.com/rust/Creating-a-server
        /// (Accessed 2025-10-05)
        const SEED_MAX: u32 = i32::MAX as u32;

        Self {
            game_params_id: uuid::Uuid::new_v4().to_string(),
            instance_id: rustctl_backend::constants::names::GAME_INSTANCE_ID.into(),
            valid_starting_from_inclusive_utc: valid_starting_from_inclusive_utc.to_owned(),
            world_size: world_size.to_owned(),
            world_seed: rand::random_range(0..=SEED_MAX),
            rcon_password: uuid::Uuid::new_v4().to_string(),
        }
    }
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
    pub game_launched_at_utc: chrono::DateTime<chrono::Utc>,
    pub game_healthy_at_utc: chrono::DateTime<chrono::Utc>,

    pub buildid: u32,

    pub carbon_version: crate::actors::gsc::gssm::CarbonVersion,

    pub world_size: GameWorldSize,
    pub world_seed: u32,
}

impl Wipe {
    pub fn new(
        game_launched_at_utc: &chrono::DateTime<chrono::Utc>,
        game_healthy_at_utc: &chrono::DateTime<chrono::Utc>,
        buildid: u32,
        carbon_version: &crate::actors::gsc::gssm::CarbonVersion,
        world_size: &GameWorldSize,
        world_seed: u32,
    ) -> Self {
        Self {
            game_launched_at_utc: game_launched_at_utc.to_owned(),
            game_healthy_at_utc: game_healthy_at_utc.to_owned(),
            buildid,
            carbon_version: carbon_version.to_owned(),
            world_size: world_size.to_owned(),
            world_seed,
        }
    }
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
