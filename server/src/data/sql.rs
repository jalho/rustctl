pub const CREATE_TABLES: &str = r#"
    CREATE TABLE users (
        user_id              TEXT NOT NULL PRIMARY KEY,
        steam_id             INTEGER NOT NULL,
        created_at_utc       TEXT NOT NULL,
        privileged_at_utc    TEXT NULL
    );

    CREATE TABLE game_params (
        instance_id          TEXT NOT NULL PRIMARY KEY,
        updated_at_utc       TEXT NOT NULL,

        world_size           INTEGER NOT NULL,
        world_seed           INTEGER NOT NULL,
        rcon_password        TEXT NOT NULL
    );

    CREATE TABLE wipes (
        startup_initiated_at_utc TEXT NOT NULL PRIMARY KEY,
        game_healthy_at_utc      TEXT NOT NULL,
        buildid                  INTEGER NOT NULL,
        carbon_version           TEXT NULL,
        world_size               INTEGER NOT NULL,
        world_seed               INTEGER NOT NULL
    );
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

const UPSERT_GAME_PARAMS: &str = r#"
    INSERT INTO game_params(
        instance_id,
        updated_at_utc,
        world_size,
        world_seed,
        rcon_password
    ) VALUES(
        $1,
        $2,
        $3,
        $4,
        $5
    )
    ON CONFLICT(instance_id) DO UPDATE SET
        updated_at_utc = excluded.updated_at_utc,
        world_size = excluded.world_size,
        world_seed = excluded.world_seed,
        rcon_password = excluded.rcon_password;
"#;

const SELECT_ALL_GAME_PARAMS: &str = r#"
    SELECT
        instance_id,
        updated_at_utc,
        world_size,
        world_seed,
        rcon_password
    FROM
        game_params;
"#;

const INSERT_WIPE: &str = r#"
    INSERT INTO wipes(
        startup_initiated_at_utc,
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
        startup_initiated_at_utc,
        game_healthy_at_utc,
        buildid,
        carbon_version,
        world_size,
        world_seed
    FROM
        wipes;
"#;

pub fn create_tables(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    let _created: () = connection.execute_batch(CREATE_TABLES)?;
    Ok(())
}

impl crate::data::schema::User {
    pub fn upsert_user(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let created_at_utc_str: String = self.created_at_utc.to_rfc3339();
        let privileged_at_utc_str: Option<String> = self.privileged_at_utc.map(|dt| dt.to_rfc3339());

        connection.execute(
            UPSERT_USER,
            (&self.id, &self.steam_id, &created_at_utc_str, &privileged_at_utc_str),
        )?;

        Ok(())
    }

    pub fn select_all_users(
        connection: &rusqlite::Connection,
    ) -> Result<Vec<crate::data::schema::User>, rusqlite::Error> {
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
    pub fn upsert_game_params(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let updated_at_utc_str = self.updated_at_utc.to_rfc3339();

        connection.execute(
            UPSERT_GAME_PARAMS,
            (
                &self.instance_id,
                &updated_at_utc_str,
                &self.world_size,
                &self.world_seed,
                &self.rcon_password,
            ),
        )?;

        Ok(())
    }

    pub fn select_all_game_params(
        connection: &rusqlite::Connection,
    ) -> Result<Vec<crate::data::schema::GameParams>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_GAME_PARAMS)?;

        let selection = statement.query_map([], |row| {
            let updated_at_utc_str: String = row.get(1)?;
            let updated_at_utc = chrono::DateTime::parse_from_rfc3339(&updated_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(1, "updated_at_utc".to_string(), rusqlite::types::Type::Text)
                })?
                .with_timezone(&chrono::Utc);

            Ok(crate::data::schema::GameParams {
                instance_id: row.get(0)?,
                updated_at_utc,
                world_size: row.get(2)?,
                world_seed: row.get(3)?,
                rcon_password: row.get(4)?,
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
    pub fn insert_wipe(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let startup_initiated_at_utc_str = self.startup_initiated_at_utc.to_rfc3339();
        let game_healthy_at_utc_str = self.game_healthy_at_utc.to_rfc3339();

        connection.execute(
            INSERT_WIPE,
            (
                &startup_initiated_at_utc_str,
                &game_healthy_at_utc_str,
                &self.buildid,
                &self.carbon_version,
                &self.world_size,
                &self.world_seed,
            ),
        )?;

        Ok(())
    }

    pub fn select_all_wipes(
        connection: &rusqlite::Connection,
    ) -> Result<Vec<crate::data::schema::Wipe>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_WIPES)?;

        let selection = statement.query_map([], |row| {
            let startup_initiated_at_utc_str: String = row.get(0)?;
            let startup_initiated_at_utc = chrono::DateTime::parse_from_rfc3339(&startup_initiated_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "startup_initiated_at_utc".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?
                .with_timezone(&chrono::Utc);

            let game_healthy_at_utc_str: String = row.get(1)?;
            let game_healthy_at_utc = chrono::DateTime::parse_from_rfc3339(&game_healthy_at_utc_str)
                .map_err(|err| {
                    log::error!("{err}");
                    rusqlite::Error::InvalidColumnType(0, "game_heathy_at_utc".to_string(), rusqlite::types::Type::Text)
                })?
                .with_timezone(&chrono::Utc);

            Ok(crate::data::schema::Wipe {
                startup_initiated_at_utc,
                game_healthy_at_utc,
                buildid: row.get(2)?,
                carbon_version: row.get(3)?,
                world_size: row.get(4)?,
                world_seed: row.get(5)?,
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
