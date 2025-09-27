pub struct Configuration {
    pub game_world_size: u16,
    pub game_world_seed: u32,

    pub rcon_port: u16,
    pub rcon_password: String,
    pub game_owner_steamid: String,

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
        let all_game_params: Vec<crate::data::schema::GameParams> =
            match crate::data::schema::GameParams::select_all_game_params(&self.connection) {
                Ok(n) => n,
                Err(err) => todo!("{err}"),
            };

        let game_params: Option<&crate::data::schema::GameParams> =
            all_game_params.iter().max_by_key(|params| params.updated_at_utc);

        if game_params.is_none() {
            let init: crate::data::schema::GameParams = crate::data::schema::GameParams {
                instance_id: "instance0".into(),
                updated_at_utc: chrono::Utc::now(),
                world_size: 1000,
                world_seed: 1,
                rcon_password: uuid::Uuid::new_v4().to_string(),
            };
            match crate::data::schema::GameParams::upsert_game_params(&init, &self.connection) {
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
                    let all_game_params: Vec<crate::data::schema::GameParams> =
                        match crate::data::schema::GameParams::select_all_game_params(&connection) {
                            Ok(n) => n,
                            Err(err) => todo!("{err}"),
                        };

                    let game_params: &crate::data::schema::GameParams =
                        match all_game_params.iter().max_by_key(|params| params.updated_at_utc) {
                            Some(params) => params,
                            None => todo!("no game params found"),
                        };

                    let all_users: Vec<crate::data::schema::User> =
                        match crate::data::schema::User::select_all_users(&connection) {
                            Ok(n) => n,
                            Err(err) => todo!("{err}"),
                        };

                    let privileged_users: Vec<&crate::data::schema::User> = all_users
                        .iter()
                        .filter(|user| user.privileged_at_utc.is_some())
                        .collect();

                    let admin: &crate::data::schema::User = match (privileged_users.first(), privileged_users.len()) {
                        (Some(user), 1) => user,
                        _ => {
                            todo!("privileged users count: {count}", count = privileged_users.len());
                        }
                    };

                    let config: Configuration = Configuration {
                          game_world_seed: game_params.world_seed,
                          game_world_size: game_params.world_size as u16,

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

        match crate::data::schema::check_version(&connection) {
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

        let all_users: Vec<crate::data::schema::User> = match crate::data::schema::User::select_all_users(&connection) {
            Ok(n) => n,
            Err(_err) => {
                match crate::data::sql::create_tables(&connection) {
                    Ok(_) => {
                        log::debug!("Tables created");
                    }
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }

                match crate::data::schema::User::select_all_users(&connection) {
                    Ok(n) => n,
                    Err(err) => {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        };

        let users: Vec<&crate::data::schema::User> = all_users
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
                    let new_user: crate::data::schema::User = crate::data::schema::User {
                        id: user_id.to_string(),
                        steam_id: *steam_id,
                        created_at_utc: now,
                        privileged_at_utc: Some(now),
                    };

                    if let Err(err) = crate::data::schema::User::upsert_user(&new_user, &connection) {
                        log::error!("{err}");
                        return Err(std::process::ExitCode::FAILURE);
                    }
                }
            }
        }

        let all_users: Vec<crate::data::schema::User> = match crate::data::schema::User::select_all_users(&connection) {
            Ok(n) => n,
            Err(err) => {
                log::error!("{err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };

        let users: Vec<&crate::data::schema::User> = all_users
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
