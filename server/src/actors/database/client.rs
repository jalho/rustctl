pub struct Client {
    tx_query: tokio::sync::mpsc::Sender<Query>,
}

impl Client {
    pub fn new(tx_query: tokio::sync::mpsc::Sender<Query>) -> Self {
        Self { tx_query }
    }

    pub async fn read_current_config(&mut self) -> rustctl_backend::GameParameters {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self
            .tx_query
            .send(Query::ReadCurrentConfiguration { respond_to: tx })
            .await
        {
            todo!("{err}");
        }
        let config: rustctl_backend::GameParameters = match rx.await {
            Ok(n) => n,
            Err(err) => todo!("{err}"),
        };
        config
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
    ReadCurrentConfiguration {
        respond_to: tokio::sync::oneshot::Sender<rustctl_backend::GameParameters>,
    },

    ReadLatestWipe {
        respond_to: tokio::sync::oneshot::Sender<Option<crate::data::schema::Wipe>>,
    },

    WriteGameUpdate {
        respond_to: tokio::sync::oneshot::Sender<()>,
        game_update: crate::data::schema::GameUpdate,
    },
}
