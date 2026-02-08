#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DbOp {
    InsertOnePasskey {
        tx: tokio::sync::oneshot::Sender<()>,
        timestamp: chrono::DateTime<chrono::Utc>,
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

    pub async fn insert_one_passkey(
        &mut self,
        timestamp: &chrono::DateTime<chrono::Utc>,
        passkey: &webauthn_rs::prelude::Passkey,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::InsertOnePasskey {
                tx,
                value: passkey.clone(),
                timestamp: *timestamp,
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
        let (client, connection): (
            tokio_postgres::Client,
            tokio_postgres::Connection<tokio_postgres::Socket, tokio_postgres::tls::NoTlsStream>,
        ) = tokio_postgres::connect(
            "postgresql://rustctl:rustctl@127.0.0.1:5432/postgres?connect_timeout=1",
            tokio_postgres::NoTls,
        )
        .await
        .unwrap();

        let connection_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            connection.await.unwrap();
        });

        while let Some(n) = self.rx.recv().await {
            match n {
                DbOp::InsertOnePasskey {
                    tx,
                    timestamp,
                    value,
                } => {
                    let passkey: webauthn_rs::prelude::Passkey = value;

                    let timestamp_utc: chrono::NaiveDateTime = timestamp.naive_utc();
                    let credential_id: Vec<u8> = passkey.cred_id().as_slice().to_vec();
                    let serializable: serde_json::Value = serde_json::to_value(&passkey).unwrap();

                    let inserted_count: u64 = client
                        .execute(
                            tables::passkeys::INSERT_ONE,
                            &[&timestamp_utc, &credential_id, &serializable],
                        )
                        .await
                        .unwrap();
                    assert_eq!(inserted_count, 1);

                    match tx.send(()) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }

                DbOp::SelectOnePasskeyByCredentialId { tx, value } => {
                    let credential_id: Vec<u8> = value;

                    let done: Vec<tokio_postgres::Row> = client
                        .query(
                            tables::passkeys::SELECT_ONE_BY_CREDENTIAL_ID,
                            &[&credential_id],
                        )
                        .await
                        .unwrap();

                    let found: Option<webauthn_rs::prelude::Passkey> = match done.len() {
                        0 => None,
                        1 => {
                            let row: &tokio_postgres::Row = done.first().unwrap();

                            let deserializable: serde_json::Value = row.get("passkey_json");

                            let deserialized: webauthn_rs::prelude::Passkey =
                                serde_json::from_value(deserializable).unwrap();

                            Some(deserialized)
                        }
                        2.. => todo!(),
                    };

                    match tx.send(found) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }
            }
        }

        match connection_handle.await {
            Ok(_) => {}
            Err(_) => todo!(),
        };
    }
}

mod tables {
    /// ```sql
    /// CREATE TABLE public.passkeys (
    ///   created_at_utc TIMESTAMP NOT NULL,
    ///   credential_id  BYTEA     PRIMARY KEY,
    ///   passkey_json   JSONB     NOT NULL
    /// );
    /// ```
    pub mod passkeys {
        pub const INSERT_ONE: &str = r#"INSERT INTO
    public.passkeys(
        created_at_utc,
        credential_id,
        passkey_json
    )
VALUES(
    $1,
    $2,
    $3
);"#;

        pub const SELECT_ONE_BY_CREDENTIAL_ID: &str = r#"SELECT
    created_at_utc,
    credential_id,
    passkey_json
FROM
    public.passkeys
WHERE
    credential_id = $1;"#;
    }
}
