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
            Ok(version) => log::info!("Connected SQLite version: {}", version),
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
            log::error!(
                r#"No privileged users exist and none set to be initialized: Use "--steam-id-init" or "--steam-id-append""#
            );
            return Err(std::process::ExitCode::FAILURE);
        }

        /*
         * TODO: Init or append given privileged users.
         */
        dbg!(populate_privileged_users);

        Ok(Self { connection })
    }

    fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
        connection.query_row(schema::READ_SQLITE_VERSION, [], |row| row.get(0))
    }

    fn select_all_privileged_users(connection: &rusqlite::Connection) -> Result<Vec<schema::User>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare(schema::SELECT_ALL_PRIVILEGED_USERS)?;

        let selection = statement.query_map([], |row| {
            Ok(schema::User {
                id: row.get(0)?,
                privileged_at_utc: row.get(1)?,
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
