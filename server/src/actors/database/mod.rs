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
        let version: String = match connection.query_row("SELECT sqlite_version()", [], |row| row.get(0)) {
            Ok(n) => n,
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };
        log::info!("Connected SQLite version: {}", version);

        /*
         * TODO: If tables not created, create them, and populate with init data
         *       given in CLI args.
         */
        dbg!(populate_privileged_users);
        {
            let mut statement = match connection.prepare("SELECT id FROM users") {
                Ok(n) => n,
                Err(err) => {
                    dbg!(&err);
                    log::error!("{err}");
                    return Err(std::process::ExitCode::FAILURE);
                }
            };

            let selection = match statement.query_map([], |row| Ok(User { id: row.get(0)? })) {
                Ok(n) => n,
                Err(err) => {
                    log::error!("{err}");
                    return Err(std::process::ExitCode::FAILURE);
                }
            };
            for selected in selection {
                let user: User = match selected {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                };
                dbg!(user);
            }
        }

        Ok(Self { connection })
    }
}

#[derive(Debug)]
struct User {
    id: String,
}
