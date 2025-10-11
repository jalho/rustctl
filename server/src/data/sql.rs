//! SQL implementations for the SQLite database of the application.
//!
//! # Module conventions and design goals
//!
//! - Method names should convey SQL operations, selectors and operable resources.
//!   Prefix with the SQL operation like _insert_ or _update_ and continue with
//!   resource name (like _user_) and selector (like _all_ or `_by_id`).
//!
//! - `SELECT` implementations should be _associated functions_ of the structures
//!   that they yield, i.e. not taking parameter `&self`.
//!
//! - All modifying implementations, i.e. everything else than `SELECT` (such as
//!   `INSERT` or `UPDATE`) should be _methods_ and not _associated functions_, i.e.
//!   taking parameter `&self`.
//!
//! - SQL should be kept minimal, and as much of the necessary complexity as
//!   possible should be implemented in Rust instead. For example, prefer simply
//!   selecting more than strictly necessary in SQL, and then narrowing the result
//!   in Rust, rather than making an overly complicated SQL query.
//!
//! These are only design goals, not strict rules. Try to understand the spirit and
//! follow that, instead of taking these goals too literally.
//!
//! # Error handling philosophy
//!
//! Only `check_database` returns `Result` because it's used at startup to check
//! database initialization and compatibility. All other functions either work, or
//! panic to terminate the program immediately because:
//!
//! - Database failures (backed by local filesystem) indicate either programming
//!   errors or severe platform issues that cannot be recovered from
//!
//! - There's no sensible way for the application to continue if the database is
//!   corrupt or inaccessible
//!
//! - Panicking immediately makes these terminal failures obvious rather than
//!   propagating errors that cannot be handled meaningfully
//!
//! I.e., the contract is: Check compatibility at startup, and then trust that
//! database operations work. If not, then panic to terminate, and hopefully someone
//! fixes the system or the program!
//!
//! To summarize the error handling philosophy: We want to distinguish between
//! recoverable and non-recoverable fallible operations.

#[derive(Debug)]
pub enum Error {
    NotInitialized,
    Incompatible {
        actual: crate::data::schema::AppDataSchemaVersion,
        expected: crate::data::schema::AppDataSchemaVersion,
    },
    NonRecoverableLibFailure {
        source: rusqlite::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NotInitialized => None,
            Error::Incompatible { actual: _, expected: _ } => None,
            Error::NonRecoverableLibFailure { source } => Some(source),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        let display: String = source.to_string();
        if display.contains("no such table") {
            Self::NotInitialized
        } else {
            Self::NonRecoverableLibFailure { source }
        }
    }
}

const CREATE_TABLES: &str = r#"
    CREATE TABLE app_data_schema_version (
        populated_by TEXT NOT NULL PRIMARY KEY
    );

    CREATE TABLE users (
        user_id           TEXT NOT NULL PRIMARY KEY,
        steam_id          INTEGER NOT NULL,

        created_at_utc    TEXT NOT NULL,
        privileged_at_utc TEXT NULL
    );

    CREATE TABLE game_params (
        game_params_id                    TEXT NOT NULL PRIMARY KEY,
        instance_id                       TEXT NOT NULL,
        valid_starting_from_inclusive_utc TEXT NOT NULL,

        world_size                        INTEGER NOT NULL,
        world_seed                        INTEGER NOT NULL,
        rcon_password                     TEXT NOT NULL
    );

    CREATE TABLE game_wipes (
        game_launched_at_utc TEXT NOT NULL PRIMARY KEY,
        game_healthy_at_utc  TEXT NOT NULL,

        buildid              INTEGER NOT NULL,

        carbon_version       TEXT NOT NULL,

        world_size           INTEGER NOT NULL,
        world_seed           INTEGER NOT NULL
    );

    CREATE TABLE game_updates (
        install_started_at_utc   TEXT NOT NULL,
        install_completed_at_utc TEXT NOT NULL,

        buildid_old              INTEGER NOT NULL,
        buildid_new              INTEGER NOT NULL PRIMARY KEY
    );
    "#;

const UPSERT_APP_DATA_SCHEMA_VERSION: &str = r#"
    INSERT INTO app_data_schema_version(populated_by) VALUES($1)
    ON CONFLICT(populated_by) DO UPDATE SET populated_by = excluded.populated_by;
"#;

const SELECT_APP_DATA_SCHEMA_VERSION: &str = r#"
    SELECT populated_by FROM app_data_schema_version;
"#;

const UPSERT_USER: &str = r#"
    INSERT INTO users(
        user_id,
        steam_id,
        created_at_utc,
        privileged_at_utc
    ) VALUES(
        $1,
        $2,
        $3,
        $4
    )
    ON CONFLICT(user_id) DO UPDATE SET
        steam_id = excluded.steam_id,
        created_at_utc = excluded.created_at_utc,
        privileged_at_utc = excluded.privileged_at_utc;
"#;

const SELECT_ALL_USERS: &str = r#"
    SELECT
        user_id,
        steam_id,
        created_at_utc,
        privileged_at_utc
    FROM
        users;
"#;

const INSERT_GAME_PARAMS: &str = r#"
    INSERT INTO game_params(
        game_params_id,
        instance_id,
        valid_starting_from_inclusive_utc,
        world_size,
        world_seed,
        rcon_password
    ) VALUES(
        $1,
        $2,
        $3,
        $4,
        $5,
        $6
    );
"#;

const SELECT_ALL_GAME_PARAMS: &str = r#"
    SELECT
        game_params_id,
        instance_id,
        valid_starting_from_inclusive_utc,
        world_size,
        world_seed,
        rcon_password
    FROM
        game_params;
"#;

const INSERT_WIPE: &str = r#"
    INSERT INTO game_wipes(
        game_launched_at_utc,
        game_healthy_at_utc,
        buildid,
        carbon_version,
        world_size,
        world_seed
    ) VALUES(
        $1,
        $2,
        $3,
        $4,
        $5,
        $6
    );
"#;

const SELECT_ALL_WIPES: &str = r#"
    SELECT
        game_launched_at_utc,
        game_healthy_at_utc,
        buildid,
        carbon_version,
        world_size,
        world_seed
    FROM
        game_wipes;
"#;

const INSERT_GAME_UPDATE: &str = r#"
    INSERT INTO game_updates(
        install_started_at_utc,
        install_completed_at_utc,
        buildid_old,
        buildid_new
    ) VALUES(
        $1,
        $2,
        $3,
        $4
    );
"#;

const SELECT_ALL_GAME_UPDATES: &str = r#"
    SELECT
        install_started_at_utc,
        install_completed_at_utc,
        buildid_old,
        buildid_new
    FROM
        game_updates;
"#;

pub fn create_tables(connection: &rusqlite::Connection) {
    connection
        .execute_batch(CREATE_TABLES)
        .expect("database table creation must succeed");
    let app_data_schema_version = crate::data::schema::AppDataSchemaVersion::new(env!("CARGO_PKG_VERSION"));
    app_data_schema_version.upsert_app_data_schema_version(connection);
}

impl crate::data::schema::AppDataSchemaVersion {
    pub fn upsert_app_data_schema_version(&self, connection: &rusqlite::Connection) {
        connection
            .execute(UPSERT_APP_DATA_SCHEMA_VERSION, [self.application_version.clone()])
            .expect("database upsert must succeed");
    }

    pub fn check_database(
        connection: &rusqlite::Connection,
        expected: crate::data::schema::AppDataSchemaVersion,
    ) -> Result<crate::data::schema::AppDataSchemaVersion, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_APP_DATA_SCHEMA_VERSION)?;
        let mut rows = statement.query([])?;

        if let Some(row) = rows.next()? {
            let actual: String = row.get(0)?;
            let actual: crate::data::schema::AppDataSchemaVersion =
                crate::data::schema::AppDataSchemaVersion::new(&actual);

            /*
             * WONTFIX: Implement semver-like compatibility: Allow e.g. app
             *          version 0.2.1 to use app data schema 0.2.0, i.e. patch
             *          bumps are not breaking changes.
             */
            if expected != actual {
                return Err(Error::Incompatible { actual, expected });
            }

            Ok(actual)
        } else {
            Err(Error::NonRecoverableLibFailure {
                source: rusqlite::Error::QueryReturnedNoRows,
            })
        }
    }
}

impl crate::data::schema::User {
    pub fn upsert_user(&self, connection: &rusqlite::Connection) {
        let created_at_utc_str: String = self.created_at_utc.to_rfc3339();
        let privileged_at_utc_str: Option<String> = self.privileged_at_utc.map(|dt| dt.to_rfc3339());

        connection
            .execute(
                UPSERT_USER,
                (&self.id, &self.steam_id, &created_at_utc_str, &privileged_at_utc_str),
            )
            .expect("database upsert must succeed");
    }

    pub fn select_all_users(connection: &rusqlite::Connection) -> Vec<crate::data::schema::User> {
        let mut statement: rusqlite::Statement = connection
            .prepare(SELECT_ALL_USERS)
            .expect("database query preparation must succeed");

        let selection = statement
            .query_map([], |row| {
                let privileged_at_utc_str: Option<String> = row.get(3)?;
                let privileged_at_utc = privileged_at_utc_str
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            3,
                            "privileged_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                let created_at_utc_str: String = row.get(2)?;
                let created_at_utc: chrono::DateTime<chrono::Utc> =
                    chrono::DateTime::parse_from_rfc3339(&created_at_utc_str)
                        .map_err(|err| {
                            log::error!("{err}");
                            rusqlite::Error::InvalidColumnType(
                                2,
                                "created_at_utc".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&chrono::Utc);

                Ok(crate::data::schema::User {
                    id: row.get(0)?,
                    steam_id: row.get(1)?,
                    created_at_utc,
                    privileged_at_utc,
                })
            })
            .expect("database query must succeed");

        selection
            .map(|user| user.expect("database row parsing must succeed"))
            .collect()
    }
}

impl crate::data::schema::GameParams {
    pub fn insert_game_params(&self, connection: &rusqlite::Connection) {
        let valid_starting_from_inclusive_utc_str = self.valid_starting_from_inclusive_utc.to_rfc3339();

        connection
            .execute(
                INSERT_GAME_PARAMS,
                (
                    &self.game_params_id,
                    &self.instance_id,
                    &valid_starting_from_inclusive_utc_str,
                    &self.world_size,
                    &self.world_seed,
                    &self.rcon_password,
                ),
            )
            .expect("database insert must succeed");
    }

    pub fn select_all_game_params(connection: &rusqlite::Connection) -> Vec<crate::data::schema::GameParams> {
        let mut statement: rusqlite::Statement = connection
            .prepare(SELECT_ALL_GAME_PARAMS)
            .expect("database query preparation must succeed");

        let selection = statement
            .query_map([], |row| {
                let valid_starting_from_inclusive_utc_str: String = row.get(2)?;
                let valid_starting_from_inclusive_utc =
                    chrono::DateTime::parse_from_rfc3339(&valid_starting_from_inclusive_utc_str)
                        .map_err(|err| {
                            log::error!("{err}");
                            rusqlite::Error::InvalidColumnType(
                                2,
                                "valid_starting_from_inclusive_utc".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&chrono::Utc);

                Ok(crate::data::schema::GameParams {
                    game_params_id: row.get(0)?,
                    instance_id: row.get(1)?,
                    valid_starting_from_inclusive_utc,
                    world_size: row.get(3)?,
                    world_seed: row.get(4)?,
                    rcon_password: row.get(5)?,
                })
            })
            .expect("database query must succeed");

        selection
            .map(|params| params.expect("database row parsing must succeed"))
            .collect()
    }
}

impl crate::data::schema::Wipe {
    pub fn insert_wipe(&self, connection: &rusqlite::Connection) {
        let game_launched_at_utc_str = self.game_launched_at_utc.to_rfc3339();
        let game_healthy_at_utc_str = self.game_healthy_at_utc.to_rfc3339();

        connection
            .execute(
                INSERT_WIPE,
                (
                    &game_launched_at_utc_str,
                    &game_healthy_at_utc_str,
                    &self.buildid,
                    &self.carbon_version,
                    &self.world_size,
                    &self.world_seed,
                ),
            )
            .expect("database insert must succeed");
    }

    pub fn select_all_wipes(connection: &rusqlite::Connection) -> Vec<crate::data::schema::Wipe> {
        let mut statement: rusqlite::Statement = connection
            .prepare(SELECT_ALL_WIPES)
            .expect("database query preparation must succeed");

        let selection = statement
            .query_map([], |row| {
                let game_launched_at_utc_str: String = row.get(0)?;
                let game_launched_at_utc = chrono::DateTime::parse_from_rfc3339(&game_launched_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "game_launched_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                let game_healthy_at_utc_str: String = row.get(1)?;
                let game_healthy_at_utc = chrono::DateTime::parse_from_rfc3339(&game_healthy_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            1,
                            "game_healthy_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                Ok(crate::data::schema::Wipe {
                    game_launched_at_utc,
                    game_healthy_at_utc,
                    buildid: row.get(2)?,
                    carbon_version: row.get(3)?,
                    world_size: row.get(4)?,
                    world_seed: row.get(5)?,
                })
            })
            .expect("database query must succeed");

        selection
            .map(|wipe| wipe.expect("database row parsing must succeed"))
            .collect()
    }
}

impl crate::data::schema::GameUpdate {
    pub fn insert_game_update(&self, connection: &rusqlite::Connection) {
        let install_started_at_utc_str = self.install_started_at_utc.to_rfc3339();
        let install_completed_at_utc_str = self.install_completed_at_utc.to_rfc3339();

        connection
            .execute(
                INSERT_GAME_UPDATE,
                (
                    &install_started_at_utc_str,
                    &install_completed_at_utc_str,
                    &self.buildid_old,
                    &self.buildid_new,
                ),
            )
            .expect("database insert must succeed");
    }

    pub fn select_all_game_updates(connection: &rusqlite::Connection) -> Vec<crate::data::schema::GameUpdate> {
        let mut statement: rusqlite::Statement = connection
            .prepare(SELECT_ALL_GAME_UPDATES)
            .expect("database query preparation must succeed");

        let selection = statement
            .query_map([], |row| {
                let install_started_at_utc_str: String = row.get(0)?;
                let install_started_at_utc = chrono::DateTime::parse_from_rfc3339(&install_started_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "install_started_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                let install_completed_at_utc_str: String = row.get(1)?;
                let install_completed_at_utc = chrono::DateTime::parse_from_rfc3339(&install_completed_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            1,
                            "install_completed_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                Ok(crate::data::schema::GameUpdate {
                    install_started_at_utc,
                    install_completed_at_utc,
                    buildid_old: row.get(2)?,
                    buildid_new: row.get(3)?,
                })
            })
            .expect("database query must succeed");

        selection
            .map(|update| update.expect("database row parsing must succeed"))
            .collect()
    }
}
