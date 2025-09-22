mod schema;

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn init_connect(
        populate_privileged_users: &crate::init::PopulatePrivilegedUsers,
    ) -> Result<Self, std::process::ExitCode> {
        let connection: rusqlite::Connection = match rusqlite::Connection::open(rustctl_backend::constants::paths::DB) {
            Ok(n) => n,
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };

        match Self::check_version(&connection) {
            Ok(version) => log::info!(
                r#"Connected SQLite version: {} -- File: "{file}""#,
                version,
                file = rustctl_backend::constants::paths::DB,
            ),
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        }

        let users: Vec<schema::User> = match Self::select_all_privileged_users(&connection) {
            Ok(n) => n,
            Err(_err) => {
                match Self::create_tables(&connection) {
                    Ok(_) => {
                        log::info!("Tables created");
                    }
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }

                match Self::select_all_privileged_users(&connection) {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        };

        if users.len() == 0
            && let crate::init::PopulatePrivilegedUsers::Noop = populate_privileged_users
        {
            log::error!(r#"No privileged users exist and none set to be initialized: Use "--steam-id-append""#);
            return Err(std::process::ExitCode::FAILURE);
        }

        if let crate::init::PopulatePrivilegedUsers::AppendToExisting { steam_ids } = populate_privileged_users {
            for steam_id in steam_ids {
                if let Some(existing) = users.iter().find(|n| &n.steam_id == steam_id) {
                    log::debug!("Appendable privileged user exists already: {existing}");
                } else {
                    if let Err(err) = Self::insert_one_privileged_user(&connection, steam_id) {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        }

        let users: Vec<schema::User> = match Self::select_all_privileged_users(&connection) {
            Ok(n) => n,
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };
        log::info!(
            "{count} privileged users in database: {listing}",
            count = users.len(),
            listing = users
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(", "),
        );

        Ok(Self { connection })
    }

    fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
        connection.query_row(schema::READ_SQLITE_VERSION, [], |row| row.get(0))
    }

    fn insert_one_privileged_user(connection: &rusqlite::Connection, steam_id: &u64) -> Result<(), rusqlite::Error> {
        /*
         * User.
         */
        let user_id: uuid::Uuid = {
            let id: uuid::Uuid = uuid::Uuid::new_v4();
            let privileged_at_utc: String = chrono::Utc::now().to_rfc3339();
            connection.execute(schema::INSERT_ONE_USER, (id.to_string(), privileged_at_utc))?;
            id
        };

        /*
         * Steam ID.
         */
        {
            let created_at_utc: String = chrono::Utc::now().to_rfc3339();
            connection.execute(
                schema::INSERT_ONE_STEAM_ID,
                (steam_id.to_string(), user_id.to_string(), created_at_utc),
            )?;
        }

        Ok(())
    }

    fn select_all_privileged_users(connection: &rusqlite::Connection) -> Result<Vec<schema::User>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare(schema::SELECT_ALL_PRIVILEGED_USERS)?;

        let selection = statement.query_map([], |row| {
            Ok(schema::User {
                id: row.get(0)?,
                created_at_utc: row.get(1)?,
                privileged_at_utc: row.get(2)?,
                steam_id: row.get(3)?,
            })
        })?;

        let mut users: Vec<schema::User> = Vec::new();
        for selected in selection {
            let user: schema::User = selected?;
            users.push(user);
        }

        return Ok(users);
    }

    fn create_tables(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let _created: () = connection.execute_batch(schema::CREATE_TABLES)?;
        Ok(())
    }
}
