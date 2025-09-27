pub struct Client {
    tx_query: tokio::sync::mpsc::Sender<Query>,
}

impl Client {
    pub fn new(tx_query: tokio::sync::mpsc::Sender<Query>) -> Self {
        Self { tx_query }
    }

    pub async fn get_config(&mut self) -> rustctl_backend::GameParameters {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Err(err) = self.tx_query.send(Query::ReadConfiguration { respond_to: tx }).await {
            todo!("{err}");
        }
        let config: rustctl_backend::GameParameters = match rx.await {
            Ok(n) => n,
            Err(err) => todo!("{err}"),
        };
        config
    }
}

pub enum Query {
    ReadConfiguration {
        respond_to: tokio::sync::oneshot::Sender<rustctl_backend::GameParameters>,
    },
}
