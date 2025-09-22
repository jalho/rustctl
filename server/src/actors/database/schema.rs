#[derive(Debug)]
pub struct User {
    pub id: String,
    pub created_at_utc: String,
    pub privileged_at_utc: String,
    pub steam_id: u64,
}

pub const CREATE_TABLES: &'static str = r#"
    CREATE TABLE users (
        user_id              TEXT NOT NULL PRIMARY KEY,

        privileged_at_utc    DATETIME NULL
    );

    CREATE TABLE steam_ids (
        steam_id             INTEGER NOT NULL PRIMARY KEY,

        user_id              TEXT NOT NULL,
        created_at_utc       DATETIME NOT NULL,
        FOREIGN KEY(user_id) REFERENCES users(user_id)
    );
"#;

pub const INSERT_ONE_USER: &'static str = r#"
    INSERT INTO users(
        user_id,
        privileged_at_utc
    ) VALUES(
        ?1,
        ?2
    );
"#;

pub const INSERT_ONE_STEAM_ID: &'static str = r#"
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

pub const SELECT_ALL_PRIVILEGED_USERS: &'static str = r#"
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

pub const READ_SQLITE_VERSION: &'static str = "SELECT sqlite_version()";
