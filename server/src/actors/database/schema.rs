#[derive(Debug)]
pub struct User {
    pub id: String,
    pub created_at_utc: chrono::DateTime<chrono::Utc>,
    pub privileged_at_utc: Option<chrono::DateTime<chrono::Utc>>,
    pub steam_id: u64,
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Steam ID {steam_id}", steam_id = self.steam_id)
    }
}

pub const CREATE_TABLES: &str = r#"
    CREATE TABLE users (
        user_id              TEXT NOT NULL PRIMARY KEY,
        steam_id             INTEGER NOT NULL,
        created_at_utc       TEXT NOT NULL,
        privileged_at_utc    TEXT NULL
    );
"#;

pub const INSERT_ONE_USER: &str = r#"
    INSERT INTO users(
        user_id,
        steam_id,
        created_at_utc,
        privileged_at_utc
    ) VALUES(
        ?1,
        ?2,
        ?3,
        ?4
    );
"#;

pub const SELECT_ALL_PRIVILEGED_USERS: &str = r#"
    SELECT
        user_id,
        steam_id,
        created_at_utc,
        privileged_at_utc
    FROM
        users
    WHERE
        privileged_at_utc IS NOT NULL
"#;

pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";
