//! Conventions and design goals
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

#[derive(Debug)]
pub enum Error {
    /// The database has not yet been initialized, i.e. the tables have not yet
    /// been created. This is not a terminal failure point: Instead, the tables
    /// should simply be created and initialized.
    NotInitialized { seeked_file_absolute_path: String },

    /// The existing database with some tables is not compatible with the
    /// current version of the application. This is a terminal failure point
    /// that cannot be recovered from automatically.
    Incompatible {
        actual: crate::data::schema::AppDataSchemaVersion,
        expected: crate::data::schema::AppDataSchemaVersion,
    },

    /// Any failure cases propagated from the underlying database client
    /// library. If such failure is propagated through, it should be considered
    /// a non-recoverable failure.
    NonRecoverableLibFailure { source: rusqlite::Error },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NotInitialized {
                seeked_file_absolute_path: _,
            } => None,
            Error::Incompatible { actual: _, expected: _ } => None,
            Error::NonRecoverableLibFailure { source } => Some(source),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        Self::NonRecoverableLibFailure { source }
    }
}

pub const CREATE_TABLES: &str = r#"
    CREATE TABLE app_data_schema_version (
        populated_by TEXT NOT NULL PRIMARY KEY
    );

    CREATE TABLE users (
        user_id              TEXT NOT NULL PRIMARY KEY,
        steam_id             INTEGER NOT NULL,

        created_at_utc       TEXT NOT NULL,
        privileged_at_utc    TEXT NULL
    );

    CREATE TABLE game_params (
        game_params_id                       TEXT NOT NULL PRIMARY KEY,
        instance_id                          TEXT NOT NULL,
        valid_starting_from_inclusive_utc    TEXT NOT NULL,

        world_size                           INTEGER NOT NULL,
        world_seed                           INTEGER NOT NULL,
        rcon_password                        TEXT NOT NULL
    );

    CREATE TABLE game_wipes (
        game_install_or_update_initiated_at_utc TEXT NOT NULL,
        game_startup_initiated_at_utc           TEXT NOT NULL PRIMARY KEY,
        game_healthy_at_utc                     TEXT NOT NULL,

        buildid                                 INTEGER NOT NULL,

        carbon_version                          TEXT NULL,

        world_size                              INTEGER NOT NULL,
        world_seed                              INTEGER NOT NULL
    );

    CREATE TABLE game_updates (
        detected_at_utc  TEXT NOT NULL,
        installed_at_utc TEXT NOT NULL,

        buildid_old      INTEGER NOT NULL,
        buildid_new      INTEGER NOT NULL PRIMARY KEY
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
        game_install_or_update_initiated_at_utc,
        game_startup_initiated_at_utc,
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
        $6,
        $7
    );
"#;

const SELECT_ALL_WIPES: &str = r#"
    SELECT
        game_install_or_update_initiated_at_utc,
        game_startup_initiated_at_utc,
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
        detected_at_utc,
        installed_at_utc,
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
        detected_at_utc,
        installed_at_utc,
        buildid_old,
        buildid_new
    FROM
        game_updates;
"#;

pub fn create_tables(connection: &rusqlite::Connection) -> Result<(), Error> {
    let _created: () = connection.execute_batch(CREATE_TABLES)?;
    let app_data_schema_version: &'static str = env!("CARGO_PKG_VERSION");
    let app_data_schema_version = crate::data::schema::AppDataSchemaVersion(app_data_schema_version.to_owned());
    app_data_schema_version.upsert_app_data_schema_version(connection)?;
    Ok(())
}

impl crate::data::schema::AppDataSchemaVersion {
    pub fn upsert_app_data_schema_version(&self, connection: &rusqlite::Connection) -> Result<(), Error> {
        let value: String = self.0.clone();
        connection.execute(UPSERT_APP_DATA_SCHEMA_VERSION, [value])?;
        Ok(())
    }

    pub fn select_app_data_schema_version(
        connection: &rusqlite::Connection,
    ) -> Result<crate::data::schema::AppDataSchemaVersion, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_APP_DATA_SCHEMA_VERSION)?;
        let mut rows = statement.query([])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            let app_data_schema_version = crate::data::schema::AppDataSchemaVersion(value);
            Ok(app_data_schema_version)
        } else {
            todo!("app data schema version not found");
        }
    }
}

impl crate::data::schema::User {
    pub fn upsert_user(&self, connection: &rusqlite::Connection) -> Result<(), Error> {
        let created_at_utc_str: String = self.created_at_utc.to_rfc3339();
        let privileged_at_utc_str: Option<String> = self.privileged_at_utc.map(|dt| dt.to_rfc3339());

        connection.execute(
            UPSERT_USER,
            (&self.id, &self.steam_id, &created_at_utc_str, &privileged_at_utc_str),
        )?;

        Ok(())
    }

    pub fn select_all_users(connection: &rusqlite::Connection) -> Result<Vec<crate::data::schema::User>, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_USERS)?;

        let selection = statement.query_map([], |row| {
            let privileged_at_utc_str: Option<String> = row.get(3)?;
            let privileged_at_utc = privileged_at_utc_str
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(3, "privileged_at_utc".to_string(), rusqlite::types::Type::Text)
                })?
                .map(|dt| dt.with_timezone(&chrono::Utc));

            let created_at_utc_str: String = row.get(2)?;
            let created_at_utc: chrono::DateTime<chrono::Utc> =
                chrono::DateTime::parse_from_rfc3339(&created_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(2, "created_at_utc".to_string(), rusqlite::types::Type::Text)
                    })?
                    .with_timezone(&chrono::Utc);

            Ok(crate::data::schema::User {
                id: row.get(0)?,
                steam_id: row.get(1)?,
                created_at_utc,
                privileged_at_utc,
            })
        })?;

        let mut users: Vec<crate::data::schema::User> = Vec::new();
        for selected in selection {
            let user: crate::data::schema::User = selected?;
            users.push(user);
        }

        Ok(users)
    }
}

impl crate::data::schema::GameParams {
    pub fn insert_game_params(&self, connection: &rusqlite::Connection) -> Result<(), Error> {
        let valid_starting_from_inclusive_utc_str = self.valid_starting_from_inclusive_utc.to_rfc3339();

        connection.execute(
            INSERT_GAME_PARAMS,
            (
                &self.game_params_id,
                &self.instance_id,
                &valid_starting_from_inclusive_utc_str,
                &self.world_size,
                &self.world_seed,
                &self.rcon_password,
            ),
        )?;

        Ok(())
    }

    pub fn select_all_game_params(
        connection: &rusqlite::Connection,
    ) -> Result<Vec<crate::data::schema::GameParams>, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_GAME_PARAMS)?;

        let selection = statement.query_map([], |row| {
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
        })?;

        let mut game_params: Vec<crate::data::schema::GameParams> = Vec::new();
        for selected in selection {
            let params: crate::data::schema::GameParams = selected?;
            game_params.push(params);
        }

        Ok(game_params)
    }
}

impl crate::data::schema::Wipe {
    pub fn insert_wipe(&self, connection: &rusqlite::Connection) -> Result<(), Error> {
        let game_install_or_update_initiated_at_utc_str = self.game_install_or_update_initiated_at_utc.to_rfc3339();
        let game_startup_initiated_at_utc_str = self.game_startup_initiated_at_utc.to_rfc3339();
        let game_healthy_at_utc_str = self.game_healthy_at_utc.to_rfc3339();

        connection.execute(
            INSERT_WIPE,
            (
                &game_install_or_update_initiated_at_utc_str,
                &game_startup_initiated_at_utc_str,
                &game_healthy_at_utc_str,
                &self.buildid,
                &self.carbon_version,
                &self.world_size,
                &self.world_seed,
            ),
        )?;

        Ok(())
    }

    pub fn select_all_wipes(connection: &rusqlite::Connection) -> Result<Vec<crate::data::schema::Wipe>, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_WIPES)?;

        let selection = statement.query_map([], |row| {
            let game_install_or_update_initiated_at_utc_str: String = row.get(0)?;
            let game_install_or_update_initiated_at_utc =
                chrono::DateTime::parse_from_rfc3339(&game_install_or_update_initiated_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "game_install_or_update_initiated_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

            let game_startup_initiated_at_utc_str: String = row.get(1)?;
            let game_startup_initiated_at_utc =
                chrono::DateTime::parse_from_rfc3339(&game_startup_initiated_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(
                            1,
                            "game_startup_initiated_at_utc".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&chrono::Utc);

            let game_healthy_at_utc_str: String = row.get(2)?;
            let game_healthy_at_utc = chrono::DateTime::parse_from_rfc3339(&game_healthy_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(
                        2,
                        "game_healthy_at_utc".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&chrono::Utc);

            Ok(crate::data::schema::Wipe {
                game_install_or_update_initiated_at_utc,
                game_startup_initiated_at_utc,
                game_healthy_at_utc,
                buildid: row.get(3)?,
                carbon_version: row.get(4)?,
                world_size: row.get(5)?,
                world_seed: row.get(6)?,
            })
        })?;

        let mut wipes: Vec<crate::data::schema::Wipe> = Vec::new();
        for selected in selection {
            let wipe: crate::data::schema::Wipe = selected?;
            wipes.push(wipe);
        }

        Ok(wipes)
    }
}

impl crate::data::schema::GameUpdate {
    pub fn insert_game_update(&self, connection: &rusqlite::Connection) -> Result<(), Error> {
        let detected_at_utc_str = self.detected_at_utc.to_rfc3339();
        let installed_at_utc_str = self.installed_at_utc.to_rfc3339();

        connection.execute(
            INSERT_GAME_UPDATE,
            (
                &detected_at_utc_str,
                &installed_at_utc_str,
                &self.buildid_old,
                &self.buildid_new,
            ),
        )?;

        Ok(())
    }

    pub fn select_all_game_updates(
        connection: &rusqlite::Connection,
    ) -> Result<Vec<crate::data::schema::GameUpdate>, Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_GAME_UPDATES)?;

        let selection = statement.query_map([], |row| {
            let detected_at_utc_str: String = row.get(0)?;
            let detected_at_utc = chrono::DateTime::parse_from_rfc3339(&detected_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(0, "detected_at_utc".to_string(), rusqlite::types::Type::Text)
                })?
                .with_timezone(&chrono::Utc);

            let installed_at_utc_str: String = row.get(1)?;
            let installed_at_utc = chrono::DateTime::parse_from_rfc3339(&installed_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(1, "installed_at_utc".to_string(), rusqlite::types::Type::Text)
                })?
                .with_timezone(&chrono::Utc);

            Ok(crate::data::schema::GameUpdate {
                detected_at_utc,
                installed_at_utc,
                buildid_old: row.get(2)?,
                buildid_new: row.get(3)?,
            })
        })?;

        let mut game_updates: Vec<crate::data::schema::GameUpdate> = Vec::new();
        for selected in selection {
            let update: crate::data::schema::GameUpdate = selected?;
            game_updates.push(update);
        }

        Ok(game_updates)
    }
}
