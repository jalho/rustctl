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

        /*
         * TODO: If tables not created, create them, and populate with init data
         *       given in CLI args.
         */
        dbg!(populate_privileged_users);
        let users: Vec<User> = match Self::select_all_privileged_users(&connection) {
            Ok(n) => n,
            Err(err) => {
                todo!();
            },
        };
        dbg!(users);

        Ok(Self { connection })
    }

    fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
        connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))
    }

    fn select_all_privileged_users(connection: &rusqlite::Connection) -> Result<Vec<User>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare("SELECT id FROM users")?;

        let selection = statement.query_map([], |row| Ok(User { id: row.get(0)? }))?;

        let mut users: Vec<User> = Vec::new();
        for selected in selection {
            let user: User = selected?;
            users.push(user);
        }

        return Ok(users);
    }

    fn create_tables(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let _created: () = connection.execute_batch(TABLES)?;
        Ok(())
    }
}

#[derive(Debug)]
struct User {
    id: String,
}

const TABLES: &'static str = r#"CREATE TABLE users (
    id                   TEXT NOT NULL PRIMARY KEY,
    privileged_at_utc    DATETIME NULL
);
CREATE TABLE alt_ids (
    id                   TEXT NOT NULL PRIMARY KEY,
    steam_id             INTEGER NOT NULL,
    user_id              TEXT NOT NULL,
    created_at_utc       DATETIME NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id)
);"#;
