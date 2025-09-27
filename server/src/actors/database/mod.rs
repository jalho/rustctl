mod schema {
    #[derive(Debug)]
    pub struct User {
        pub id: String,
        pub created_at_utc: chrono::DateTime<chrono::Utc>,
        pub privileged_at_utc: Option<chrono::DateTime<chrono::Utc>>,
        pub steam_id: u64,
    }

    impl std::fmt::Display for User {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "user ID {user_id}: Steam ID {steam_id} (created {created_at}{privileged_at})",
                steam_id = self.steam_id,
                user_id = self.id,
                created_at = self.created_at_utc.date_naive(),
                privileged_at = match self.privileged_at_utc {
                    Some(instant) => format!(", privileged {instant}", instant = instant.date_naive()),
                    None => "".into(),
                }
            )
        }
    }

    impl User {
        pub fn upsert_user(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
            let created_at_utc_str: String = self.created_at_utc.to_rfc3339();
            let privileged_at_utc_str: Option<String> = self.privileged_at_utc.map(|dt| dt.to_rfc3339());

            connection.execute(
                UPSERT_USER,
                (&self.id, &self.steam_id, &created_at_utc_str, &privileged_at_utc_str),
            )?;

            Ok(())
        }

        pub fn select_all_users(connection: &rusqlite::Connection) -> Result<Vec<User>, rusqlite::Error> {
            let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_USERS)?;

            let selection = statement.query_map([], |row| {
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

                Ok(User {
                    id: row.get(0)?,
                    steam_id: row.get(1)?,
                    created_at_utc,
                    privileged_at_utc,
                })
            })?;

            let mut users: Vec<User> = Vec::new();
            for selected in selection {
                let user: User = selected?;
                users.push(user);
            }

            Ok(users)
        }
    }

    #[derive(Debug)]
    pub struct GameParams {
        pub instance_id: String,
        pub updated_at_utc: chrono::DateTime<chrono::Utc>,
        pub world_size: u32,
        pub world_seed: u32,
        pub rcon_password: String,
    }

    impl std::fmt::Display for GameParams {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "instance {instance_id}: world size {world_size}, seed {world_seed} (updated {updated_at})",
                instance_id = self.instance_id,
                world_size = self.world_size,
                world_seed = self.world_seed,
                updated_at = self.updated_at_utc.date_naive()
            )
        }
    }

    impl GameParams {
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

        pub fn select_all_game_params(connection: &rusqlite::Connection) -> Result<Vec<GameParams>, rusqlite::Error> {
            let mut statement: rusqlite::Statement = connection.prepare(SELECT_ALL_GAME_PARAMS)?;

            let selection = statement.query_map([], |row| {
                let updated_at_utc_str: String = row.get(1)?;
                let updated_at_utc = chrono::DateTime::parse_from_rfc3339(&updated_at_utc_str)
                    .map_err(|err| {
                        log::error!("{err}");
                        rusqlite::Error::InvalidColumnType(1, "updated_at_utc".to_string(), rusqlite::types::Type::Text)
                    })?
                    .with_timezone(&chrono::Utc);

                Ok(GameParams {
                    instance_id: row.get(0)?,
                    updated_at_utc,
                    world_size: row.get(2)?,
                    world_seed: row.get(3)?,
                    rcon_password: row.get(4)?,
                })
            })?;

            let mut game_params: Vec<GameParams> = Vec::new();
            for selected in selection {
                let params: GameParams = selected?;
                game_params.push(params);
            }

            Ok(game_params)
        }
    }

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
"#;

    pub const UPSERT_USER: &str = r#"
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

    pub const SELECT_ALL_USERS: &str = r#"
    SELECT
        user_id,
        steam_id,
        created_at_utc,
        privileged_at_utc
    FROM
        users;
"#;

    pub const UPSERT_GAME_PARAMS: &str = r#"
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

    pub const SELECT_ALL_GAME_PARAMS: &str = r#"
    SELECT
        instance_id,
        updated_at_utc,
        world_size,
        world_seed,
        rcon_password
    FROM
        game_params;
"#;

    pub const READ_SQLITE_VERSION: &str = "SELECT sqlite_version()";

    pub fn create_tables(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let _created: () = connection.execute_batch(CREATE_TABLES)?;
        Ok(())
    }

    pub fn check_version(connection: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
        connection.query_row(READ_SQLITE_VERSION, [], |row| row.get(0))
    }
}

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
    pub async fn work(self) -> Summary {
        let all_game_params: Vec<schema::GameParams> =
            match schema::GameParams::select_all_game_params(&self.connection) {
                Ok(n) => n,
                Err(err) => todo!("{err}"),
            };

        let game_params: Option<&schema::GameParams> =
            all_game_params.iter().max_by_key(|params| params.updated_at_utc);

        if game_params.is_none() {
            let init: schema::GameParams = schema::GameParams {
                instance_id: "instance0".into(),
                updated_at_utc: chrono::Utc::now(),
                world_size: 1000,
                world_seed: 1,
                rcon_password: uuid::Uuid::new_v4().to_string(),
            };
            match init.upsert_game_params(&self.connection) {
                Ok(_) => log::info!("Initialized game params"),
                Err(err) => todo!("{err}"),
            }
        }

        let job = Self::handle_queries(self.connection, self.rx_query);

        self.ctoken.run_until_cancelled(job).await;

        Summary
    }

    async fn handle_queries(
        connection: rusqlite::Connection,
        mut rx_query: tokio::sync::mpsc::Receiver<client::Query>,
    ) {
        loop {
            let query: client::Query = match rx_query.recv().await {
                Some(n) => n,
                None => todo!(),
            };
            match query {
                client::Query::ReadConfiguration { respond_to } => {
                    let all_game_params: Vec<schema::GameParams> =
                        match schema::GameParams::select_all_game_params(&connection) {
                            Ok(n) => n,
                            Err(err) => todo!("{err}"),
                        };

                    let game_params: &schema::GameParams =
                        match all_game_params.iter().max_by_key(|params| params.updated_at_utc) {
                            Some(params) => params,
                            None => todo!("no game params found"),
                        };

                    let all_users: Vec<schema::User> = match schema::User::select_all_users(&connection) {
                        Ok(n) => n,
                        Err(err) => todo!("{err}"),
                    };

                    let privileged_users: Vec<&schema::User> = all_users
                        .iter()
                        .filter(|user| user.privileged_at_utc.is_some())
                        .collect();

                    let admin: &schema::User = match (privileged_users.first(), privileged_users.len()) {
                        (Some(user), 1) => user,
                        _ => {
                            todo!("privileged users count: {count}", count = privileged_users.len());
                        }
                    };

                    let config: Configuration = Configuration {
                          game_world_seed: game_params.world_seed,
                          game_world_size: game_params.world_size as u16, // TODO: Remove cast: Declare the type as u16!

                          rcon_port: 28016,
                          rcon_password: game_params.rcon_password.clone(),
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

        match schema::check_version(&connection) {
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

        let all_users: Vec<schema::User> = match schema::User::select_all_users(&connection) {
            Ok(n) => n,
            Err(_err) => {
                match schema::create_tables(&connection) {
                    Ok(_) => {
                        log::debug!("Tables created");
                    }
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }

                match schema::User::select_all_users(&connection) {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        };

        let users: Vec<&schema::User> = all_users
            .iter()
            .filter(|user| user.privileged_at_utc.is_some())
            .collect();

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
                } else {
                    let user_id: uuid::Uuid = uuid::Uuid::new_v4();
                    let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
                    let new_user: schema::User = schema::User {
                        id: user_id.to_string(),
                        steam_id: *steam_id,
                        created_at_utc: now,
                        privileged_at_utc: Some(now),
                    };

                    if let Err(err) = new_user.upsert_user(&connection) {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        }

        let all_users: Vec<schema::User> = match schema::User::select_all_users(&connection) {
            Ok(n) => n,
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };

        let users: Vec<&schema::User> = all_users
            .iter()
            .filter(|user| user.privileged_at_utc.is_some())
            .collect();

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
}
