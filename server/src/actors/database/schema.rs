#[derive(Debug)]
pub struct User {
    pub id: String,
    pub created_at_utc: chrono::DateTime<chrono::Utc>,
    pub privileged_at_utc: chrono::DateTime<chrono::Utc>,
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

        privileged_at_utc    TEXT NULL
    );

    CREATE TABLE steam_ids (
        steam_id             INTEGER NOT NULL PRIMARY KEY,

        user_id              TEXT NOT NULL,
        created_at_utc       TEXT NOT NULL,
        FOREIGN KEY(user_id) REFERENCES users(user_id)
    );
"#;

pub const INSERT_ONE_USER: &str = r#"
    INSERT INTO users(
        user_id,
        privileged_at_utc
    ) VALUES(
        ?1,
        ?2
    );
"#;

pub const INSERT_ONE_STEAM_ID: &str = r#"
    INSERT INTO steam_ids(
        steam_id,
        user_id,
        created_at_utc
    ) VALUES(
        ?1,
        ?2,
        ?3
    );
"#;

pub const SELECT_ALL_PRIVILEGED_USERS: &str = r#"
    SELECT
        u.user_id,
        s.created_at_utc,
        u.privileged_at_utc,
        s.steam_id
    FROM
        users u
    JOIN
        steam_ids s
        ON u.user_id = s.user_id
    WHERE
        privileged_at_utc IS NOT NULL
"#;

pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";
