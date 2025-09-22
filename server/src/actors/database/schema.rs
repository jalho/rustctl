#[derive(Debug)]
pub struct User {
    pub id: String,
    pub privileged_at_utc: String,
}

pub const CREATE_TABLES: &'static str = r#"
    CREATE TABLE users (
        id                   TEXT NOT NULL PRIMARY KEY,
        privileged_at_utc    DATETIME NULL
    );

    CREATE TABLE alt_ids (
        id                   TEXT NOT NULL PRIMARY KEY,
        steam_id             INTEGER NOT NULL,
        user_id              TEXT NOT NULL,
        created_at_utc       DATETIME NOT NULL,
        FOREIGN KEY(user_id) REFERENCES users(id)
    );
"#;

pub const SELECT_ALL_PRIVILEGED_USERS: &'static str = r#"
    SELECT
        id,
        privileged_at_utc
    FROM
        users
    WHERE
        privileged_at_utc IS NOT NULL
"#;

pub const READ_SQLITE_VERSION: &'static str = "SELECT sqlite_version()";
