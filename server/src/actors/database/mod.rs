mod schema;

pub struct Configuration {
    pub game_world_size: u16,
    pub game_world_seed: u32,

    pub rcon_port: u16,
    pub rcon_password: String,
    pub game_owner_steamid: String,

    /// URL from where _Carbon Modding Framework_ shall be downloaded from.
    ///
    /// For example:
    /// ```
    /// "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz"
    /// ```
    pub carbon_download_url: String,

    pub game_name: String,
    pub game_description: String,
    pub game_url_home: String,
    pub game_url_header: String,
    pub game_url_logo: String,
}

impl Configuration {
    pub fn get_installer_args(&self) -> Vec<&'static str> {
        vec![
            "+login",
            "anonymous",
            /*
             * WONTFIX: "force_install_dir" doesn't really "force" anything:
             *          Instead, SteamCMD seems to just create a new directory
             *          tree in "~/.local/share/Steam/" if it cannot access
             *          the given "force_install_dir".
             *
             *          Behavior observed in `apt` packaged version:
             *          - Package: steamcmd:i386
             *          - Version: 0~20180105-5 (latest as of July 2025)
             *          - Section: non-free/games
             *          - Maintainer: Debian Games Team
             */
            "+force_install_dir",
            rustctl_backend::constants::paths::ROOT_DIR,
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ]
    }

    pub fn get_rcon_connection_string(&self) -> String {
        format!(
            "ws://127.0.0.1:{port}/{password}",
            port = self.rcon_port,
            password = self.rcon_password,
        )
    }
}

pub mod client {
    pub struct Client {
        tx_query: tokio::sync::mpsc::Sender<Query>,
    }

    impl Client {
        pub fn new(tx_query: tokio::sync::mpsc::Sender<Query>) -> Self {
            Self { tx_query }
        }

        pub async fn get_config(&mut self) -> crate::actors::database::Configuration {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if let Err(err) = self.tx_query.send(Query::ReadConfiguration { respond_to: tx }).await {
                todo!("{err}");
            }
            let config: crate::actors::database::Configuration = match rx.await {
                Ok(n) => n,
                Err(err) => todo!("{err}"),
            };
            config
        }
    }

    pub enum Query {
        ReadConfiguration {
            respond_to: tokio::sync::oneshot::Sender<crate::actors::database::Configuration>,
        },
    }
}

pub struct Summary;

pub struct Database {
    ctoken: tokio_util::sync::CancellationToken,

    connection: rusqlite::Connection,
    rx_query: tokio::sync::mpsc::Receiver<client::Query>,
}

impl Database {
    pub async fn work(mut self) -> Summary {
        let job = async {
            loop {
                let query: client::Query = match self.rx_query.recv().await {
                    Some(n) => n,
                    None => todo!(),
                };
                match query {
                    client::Query::ReadConfiguration { respond_to } => {
                        let privileged_users: Vec<schema::User> =
                            match Self::select_all_privileged_users(&self.connection) {
                                Ok(n) => n,
                                Err(_) => todo!(),
                            };

                        let admin: &schema::User = match (privileged_users.first(), privileged_users.len()) {
                            (Some(user), 1) => user,
                            _ => {
                                todo!("privileged users count: {count}", count = privileged_users.len());
                            }
                        };

                        /*
                         * TODO: Read the rest of the values from the database too!
                         */
                        let config: Configuration = Configuration {
                          game_world_seed: 1,
                          game_world_size: 1000, // minimum world size AFAIK

                          rcon_port: 28016,
                          rcon_password: uuid::Uuid::new_v4().to_string(),
                          game_owner_steamid: admin.steam_id.to_string(),

                          carbon_download_url: "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz".to_string(),

                          game_name: "rustctl".to_string(),
                          game_description: "rustctl managed server".to_string(),
                          game_url_home: "https://github.com/jalho/rustctl".to_string(),
                          game_url_header: "https://upload.wikimedia.org/wikipedia/commons/thumb/c/c1/Vexillum_aboense.jpg/1280px-Vexillum_aboense.jpg".to_string(),
                          game_url_logo: "https://upload.wikimedia.org/wikipedia/commons/thumb/b/bc/Flag_of_Finland.svg/60px-Flag_of_Finland.svg.png".to_string(),
                        };
                        if respond_to.send(config).is_err() {
                            todo!();
                        }
                    }
                }
            }
        };

        self.ctoken.run_until_cancelled(job).await;

        Summary
    }

    pub fn init_connect(
        ctoken: tokio_util::sync::CancellationToken,
        populate_privileged_users: &crate::init::PopulatePrivilegedUsers,
        rx_query: tokio::sync::mpsc::Receiver<client::Query>,
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
                        log::debug!("Tables created");
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

        if users.is_empty()
            && let crate::init::PopulatePrivilegedUsers::Noop = populate_privileged_users
        {
            log::error!(r#"No privileged users exist and none set to be initialized: Use "--steam-id-append""#);
            return Err(std::process::ExitCode::FAILURE);
        }

        if let crate::init::PopulatePrivilegedUsers::AppendToExisting { steam_ids } = populate_privileged_users {
            for steam_id in steam_ids {
                if let Some(existing) = users.iter().find(|n| &n.steam_id == steam_id) {
                    log::debug!("Appendable privileged user exists already: {existing}");
                } else if let Err(err) = Self::insert_one_privileged_user(&connection, steam_id) {
                    log::error!("{err}");
                    return Err(std::process::ExitCode::FAILURE);
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
            listing = users.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", "),
        );

        Ok(Self {
            connection,
            ctoken,
            rx_query,
        })
    }

    fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
        connection.query_row(schema::READ_SQLITE_VERSION, [], |row| row.get(0))
    }

    fn insert_one_privileged_user(connection: &rusqlite::Connection, steam_id: &u64) -> Result<(), rusqlite::Error> {
        let user_id: uuid::Uuid = uuid::Uuid::new_v4();
        let created_at_utc: String = chrono::Utc::now().to_rfc3339();
        let privileged_at_utc: String = chrono::Utc::now().to_rfc3339();

        connection.execute(
            schema::INSERT_ONE_USER,
            (
                user_id.to_string(),
                steam_id.to_string(),
                created_at_utc,
                privileged_at_utc,
            ),
        )?;

        Ok(())
    }

    fn select_all_privileged_users(connection: &rusqlite::Connection) -> Result<Vec<schema::User>, rusqlite::Error> {
        let mut statement: rusqlite::Statement = connection.prepare(schema::SELECT_ALL_PRIVILEGED_USERS)?;

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

            Ok(schema::User {
                id: row.get(0)?,
                created_at_utc: row.get(2)?,
                privileged_at_utc,
                steam_id: row.get(1)?,
            })
        })?;

        let mut users: Vec<schema::User> = Vec::new();
        for selected in selection {
            let user: schema::User = selected?;
            users.push(user);
        }

        Ok(users)
    }

    fn create_tables(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let _created: () = connection.execute_batch(schema::CREATE_TABLES)?;
        Ok(())
    }
}
