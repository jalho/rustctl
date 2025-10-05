pub struct Client {
    tx_query: tokio::sync::mpsc::Sender<Query>,
}

impl Client {
    pub fn new(tx_query: tokio::sync::mpsc::Sender<Query>) -> Self {
        Self { tx_query }
    }

    pub async fn read_users_privileged(&mut self) -> Vec<crate::data::schema::User> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self.tx_query.send(Query::ReadUsers { respond_to: tx }).await {
            todo!("{err}");
        }
        let users_all: Vec<crate::data::schema::User> = match rx.await {
            Ok(n) => n,
            Err(err) => todo!("{err}"),
        };
        let users_privileged: Vec<crate::data::schema::User> = users_all
            .into_iter()
            .filter(|n| n.privileged_at_utc.is_some())
            .collect::<Vec<crate::data::schema::User>>();
        users_privileged
    }

    pub async fn read_game_params(
        &mut self,
        for_instant: &chrono::DateTime<chrono::Utc>,
    ) -> Option<crate::data::schema::GameParams> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self
            .tx_query
            .send(Query::ReadGameParams {
                respond_to: tx,
                for_instant: *for_instant,
            })
            .await
        {
            todo!("{err}");
        }
        let config: Option<crate::data::schema::GameParams> = match rx.await {
            Ok(n) => n,
            Err(err) => todo!("{err}"),
        };
        config
    }

    pub async fn write_game_params(&mut self, game_params: &crate::data::schema::GameParams) -> () {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self
            .tx_query
            .send(Query::WriteGameParams {
                respond_to: tx,
                game_params: game_params.clone(),
            })
            .await
        {
            todo!("{err}");
        }
        if let Err(err) = rx.await {
            todo!("{err}");
        };
    }

    pub async fn read_latest_wipe(&mut self) -> Option<crate::data::schema::Wipe> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self.tx_query.send(Query::ReadLatestWipe { respond_to: tx }).await {
            todo!("{err}");
        }
        let latest_wipe: Option<crate::data::schema::Wipe> = match rx.await {
            Ok(n) => n,
            Err(err) => todo!("{err}"),
        };
        latest_wipe
    }

    pub async fn write_game_update(&mut self, game_update: &crate::data::schema::GameUpdate) -> () {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self
            .tx_query
            .send(Query::WriteGameUpdate {
                respond_to: tx,
                game_update: game_update.clone(),
            })
            .await
        {
            todo!("{err}");
        }
        if let Err(err) = rx.await {
            todo!("{err}");
        };
    }
}

pub enum Query {
    ReadUsers {
        respond_to: tokio::sync::oneshot::Sender<Vec<crate::data::schema::User>>,
    },
    WriteUser {
        respond_to: tokio::sync::oneshot::Sender<()>,
        user: crate::data::schema::User,
    },

    ReadGameParams {
        respond_to: tokio::sync::oneshot::Sender<Option<crate::data::schema::GameParams>>,
        for_instant: chrono::DateTime<chrono::Utc>,
    },
    WriteGameParams {
        respond_to: tokio::sync::oneshot::Sender<()>,
        game_params: crate::data::schema::GameParams,
    },

    ReadLatestWipe {
        respond_to: tokio::sync::oneshot::Sender<Option<crate::data::schema::Wipe>>,
    },
    WriteGameUpdate {
        respond_to: tokio::sync::oneshot::Sender<()>,
        game_update: crate::data::schema::GameUpdate,
    },
}
