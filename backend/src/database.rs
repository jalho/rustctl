#[derive(Debug)]
pub enum DbOp {
    InsertOnePasskey {
        tx: tokio::sync::oneshot::Sender<()>,
        value: webauthn_rs::prelude::Passkey,
    },
    SelectOnePasskeyByCredentialId {
        tx: tokio::sync::oneshot::Sender<Option<webauthn_rs::prelude::Passkey>>,
        value: Vec<u8>,
    },
}

#[derive(Clone)]
pub struct Client {
    tx: tokio::sync::mpsc::Sender<DbOp>,
}

impl Client {
    pub fn new(tx: tokio::sync::mpsc::Sender<DbOp>) -> Self {
        Self { tx }
    }

    pub async fn insert_one_passkey(&mut self, passkey: &webauthn_rs::prelude::Passkey) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::InsertOnePasskey {
                tx,
                value: passkey.clone(),
            })
            .await
        {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(_) => {}
            Err(_) => todo!(),
        }
    }

    pub async fn select_one_passkey_by_credential_id(
        &mut self,
        value: &[u8],
    ) -> Option<webauthn_rs::prelude::Passkey> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::SelectOnePasskeyByCredentialId {
                tx,
                value: value.to_vec(),
            })
            .await
        {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(n) => n,
            Err(_) => todo!(),
        }
    }
}

pub struct Engine {
    rx: tokio::sync::mpsc::Receiver<DbOp>,
}

impl Engine {
    pub fn new() -> (Self, Client) {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        (Self { rx }, Client::new(tx))
    }

    pub async fn handle(&mut self) -> () {
        while let Some(n) = self.rx.recv().await {
            match n {
                DbOp::InsertOnePasskey { tx, value } => {
                    /*
                     * TODO: Serialize and insert the `value` in PostgreSQL
                     *       (connection owned by `self`).
                     */
                    let value: webauthn_rs::prelude::Passkey = value;

                    match tx.send(()) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }

                DbOp::SelectOnePasskeyByCredentialId { tx, value } => {
                    /*
                     * TODO: Find by `value` from PostgreSQL (connection owned
                     *       by `self`).
                     */
                    let value: Vec<u8> = value;

                    match tx.send(None) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }
            }
        }
    }
}
