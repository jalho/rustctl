pub mod client;

pub struct Summary;

pub struct Database {
    ctoken: tokio_util::sync::CancellationToken,

    connection: rusqlite::Connection,
    rx_query: tokio::sync::mpsc::Receiver<client::Query>,
}

impl Database {
    /// The SQLite library provides a blocking API.
    pub async fn work_blocking(self) -> Summary {
        let all_game_params: Vec<crate::data::schema::GameParams> =
            crate::data::schema::GameParams::select_all_game_params(&self.connection);

        let current_time: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
        let game_params: Option<&crate::data::schema::GameParams> = all_game_params
            .iter()
            .filter(|n| n.is_active(&current_time))
            .max_by_key(|n| n.valid_starting_from_inclusive_utc);

        if game_params.is_none() {
            let init: crate::data::schema::GameParams =
                crate::data::schema::GameParams::new(&chrono::Utc::now(), 1000, 1);
            crate::data::schema::GameParams::insert_game_params(&init, &self.connection);
        }

        let job = Self::handle_queries(&self.connection, self.rx_query);

        self.ctoken.run_until_cancelled(job).await;

        Summary
    }

    async fn handle_queries(
        connection: &rusqlite::Connection,
        mut rx_query: tokio::sync::mpsc::Receiver<client::Query>,
    ) {
        loop {
            let query: client::Query = match rx_query.recv().await {
                Some(n) => n,
                None => todo!(),
            };
            match query {
                client::Query::ReadUsers { respond_to } => {
                    let users_all: Vec<crate::data::schema::User> =
                        crate::data::schema::User::select_all_users(connection);
                    if respond_to.send(users_all).is_err() {
                        todo!();
                    }
                }

                client::Query::WriteUser { respond_to, user } => {
                    user.upsert_user(connection);
                    if respond_to.send(()).is_err() {
                        todo!();
                    }
                }

                client::Query::ReadGameParams {
                    respond_to,
                    for_instant,
                } => {
                    let mut game_params_all: Vec<crate::data::schema::GameParams> =
                        crate::data::schema::GameParams::select_all_game_params(connection);
                    game_params_all.sort_by_key(|n| n.valid_starting_from_inclusive_utc);
                    let active: Option<crate::data::schema::GameParams> = game_params_all
                        .into_iter()
                        .filter(|n| n.is_active(&for_instant))
                        .next_back();

                    if respond_to.send(active).is_err() {
                        todo!();
                    }
                }

                client::Query::WriteGameParams {
                    respond_to,
                    game_params,
                } => {
                    game_params.insert_game_params(connection);

                    if respond_to.send(()).is_err() {
                        todo!();
                    }
                }

                client::Query::ReadLatestWipe { respond_to } => {
                    let mut all_wipes: Vec<crate::data::schema::Wipe> =
                        crate::data::schema::Wipe::select_all_wipes(connection);
                    all_wipes.sort_by_key(|n| n.game_healthy_at_utc);
                    let latest: Option<crate::data::schema::Wipe> = all_wipes.into_iter().last();
                    if respond_to.send(latest).is_err() {
                        todo!();
                    }
                }

                client::Query::WriteWipe { respond_to, wipe } => {
                    wipe.insert_wipe(connection);
                    if respond_to.send(()).is_err() {
                        todo!();
                    }
                }

                client::Query::WriteGameUpdate {
                    respond_to,
                    game_update,
                } => {
                    game_update.insert_game_update(connection);
                    if respond_to.send(()).is_err() {
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

        match crate::data::schema::read_sqlite_version(&connection) {
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

        let all_users = match crate::data::schema::AppDataSchemaVersion::check_database(
            &connection,
            crate::data::schema::AppDataSchemaVersion::new(env!("CARGO_PKG_VERSION")),
        ) {
            Ok(app_data_schema_version) => {
                log::info!("App data schema version: {app_data_schema_version}");
                crate::data::schema::User::select_all_users(&connection)
            }

            Err(crate::data::sql::Error::NotInitialized) => {
                crate::data::sql::create_tables(&connection);
                crate::data::schema::User::select_all_users(&connection)
            }

            Err(crate::data::sql::Error::Incompatible { actual, expected }) => {
                log::error!("Incompatible database: Expected {expected}, got {actual}");
                return Err(std::process::ExitCode::FAILURE);
            }

            Err(crate::data::sql::Error::NonRecoverableLibFailure { source }) => {
                log::error!("Unusable database: {source}");
                return Err(std::process::ExitCode::FAILURE);
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

                    crate::data::schema::User::upsert_user(&new_user, &connection);
                }
            }
        }

        let all_users: Vec<crate::data::schema::User> = crate::data::schema::User::select_all_users(&connection);

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

trait TimeWindowed {
    fn is_active(&self, window_start_inclusive: &chrono::DateTime<chrono::Utc>) -> bool;
}

impl TimeWindowed for crate::data::schema::GameParams {
    fn is_active(&self, window_start_inclusive: &chrono::DateTime<chrono::Utc>) -> bool {
        &self.valid_starting_from_inclusive_utc <= window_start_inclusive
    }
}
