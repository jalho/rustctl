#[allow(clippy::large_enum_variant)]
pub enum DbOp {
    InsertOnePasskey {
        tx: tokio::sync::oneshot::Sender<()>,
        timestamp: chrono::DateTime<chrono::Utc>,
        passkey_name: String,
        passkey: webauthn_rs::prelude::Passkey,
    },
    SelectOnePasskeyByCredentialId {
        tx: tokio::sync::oneshot::Sender<Option<queries::SelectedPasskey>>,
        value: Vec<u8>,
    },
}

pub mod queries {
    pub struct SelectedPasskey {
        pub invalidated_at: Option<chrono::DateTime<chrono::Utc>>,
        pub passkey: webauthn_rs::prelude::Passkey,
        pub passkey_name: String,
    }
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
        passkey_name: &str,
        passkey: &webauthn_rs::prelude::Passkey,
    ) -> Result<(), ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::InsertOnePasskey {
                tx,
                timestamp: *timestamp,
                passkey_name: passkey_name.to_owned(),
                passkey: passkey.clone(),
            })
            .await
        {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub async fn select_one_passkey_by_credential_id(
        &mut self,
        value: &[u8],
    ) -> Result<Option<queries::SelectedPasskey>, ()> {
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
            Ok(n) => Ok(n),
            Err(_) => Err(()),
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

    pub async fn keep_connected(&mut self) -> () {
        'reconnect: loop {
            let (mut client, connection): (
                tokio_postgres::Client,
                tokio_postgres::Connection<
                    tokio_postgres::Socket,
                    tokio_postgres::tls::NoTlsStream,
                >,
            ) = match tokio_postgres::connect(
                "postgresql://rustctl:rustctl@127.0.0.1:5432/postgres?connect_timeout=1",
                tokio_postgres::NoTls,
            )
            .await
            {
                Ok(n) => n,
                Err(err) => {
                    log::error!("{err}", err = crate::get_full_error_message(&err));
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue 'reconnect;
                }
            };

            let connection_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                if let Err(err) = connection.await {
                    log::error!("{err}", err = crate::get_full_error_message(&err));
                }
            });

            let tables_created: bool = Self::assure_tables_exist(&mut client).await;
            if tables_created {
                log::info!("Connected to new database: Tables created");
            } else {
                log::info!("Connected to existing database: Tables not created");
            }

            tokio::select!(
                _ = self.handle_queries(client) => {}
                _ = connection_handle => {}
            )
        }
    }

    async fn assure_tables_exist(client: &mut tokio_postgres::Client) -> bool {
        match client.execute(tables::passkeys::CREATE_TABLE, &[]).await {
            Ok(_) => true,
            Err(err) => {
                let msg: String = crate::get_full_error_message(&err);
                if msg.contains("already exists") {
                    false
                } else {
                    panic!("{msg}");
                }
            }
        }
    }

    async fn handle_queries(&mut self, client: tokio_postgres::Client) {
        while let Some(n) = self.rx.recv().await {
            match n {
                DbOp::InsertOnePasskey {
                    tx,
                    timestamp,
                    passkey_name,
                    passkey,
                } => {
                    let passkey_name: String = passkey_name;
                    let passkey: webauthn_rs::prelude::Passkey = passkey;

                    let timestamp_utc: chrono::NaiveDateTime = timestamp.naive_utc();
                    let credential_id: Vec<u8> = passkey.cred_id().as_slice().to_vec();
                    let credential_id_hex: String = to_hex_string(&credential_id);
                    let serializable: serde_json::Value = serde_json::to_value(&passkey).unwrap();

                    let inserted_count: u64 = match client
                        .execute(
                            tables::passkeys::INSERT_ONE,
                            &[
                                &timestamp_utc,
                                &passkey_name,
                                &credential_id_hex,
                                &serializable,
                            ],
                        )
                        .await
                    {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("{err}", err = crate::get_full_error_message(&err));
                            return;
                        }
                    };
                    assert_eq!(inserted_count, 1);

                    match tx.send(()) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }

                DbOp::SelectOnePasskeyByCredentialId { tx, value } => {
                    let credential_id: Vec<u8> = value;
                    let credential_id_hex: String = to_hex_string(&credential_id);

                    let rows: Vec<tokio_postgres::Row> = match client
                        .query(
                            tables::passkeys::SELECT_ONE_BY_CREDENTIAL_ID,
                            &[&credential_id_hex],
                        )
                        .await
                    {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("{err}", err = crate::get_full_error_message(&err));
                            return;
                        }
                    };

                    let found: Option<queries::SelectedPasskey> = match rows.len() {
                        0 => None,
                        1 => {
                            let row: &tokio_postgres::Row = rows.first().unwrap();

                            let invalidated_at: Option<chrono::NaiveDateTime> = {
                                let deserialized = row.try_get("invalidated_at_utc");
                                let value: Option<chrono::NaiveDateTime> = match deserialized {
                                    Ok(n) => n,
                                    Err(_) => todo!(),
                                };
                                value
                            };
                            let invalidated_at: Option<chrono::DateTime<chrono::Utc>> =
                                invalidated_at.map(|n| {
                                    let n: chrono::NaiveDateTime = n;
                                    let utc: chrono::DateTime<chrono::Utc> = n.and_utc();
                                    utc
                                });

                            let passkey: webauthn_rs::prelude::Passkey = {
                                let deserialized = row.try_get("passkey_json");
                                let value: serde_json::Value = match deserialized {
                                    Ok(n) => n,
                                    Err(_) => todo!(),
                                };
                                let value: webauthn_rs::prelude::Passkey =
                                    serde_json::from_value(value).unwrap();
                                value
                            };

                            let passkey_name: String = {
                                let deserialized = row.try_get("passkey_name");
                                let value: String = match deserialized {
                                    Ok(n) => n,
                                    Err(_) => todo!(),
                                };
                                value
                            };

                            Some(queries::SelectedPasskey {
                                invalidated_at,
                                passkey,
                                passkey_name,
                            })
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
    }
}

mod tables {
    pub mod passkeys {
        pub const CREATE_TABLE: &str = r#"CREATE TABLE public.passkeys (
  created_at_utc     TIMESTAMP    NOT NULL,
  passkey_name       TEXT         NOT NULL,
  credential_id_hex  VARCHAR(128) PRIMARY KEY,
  passkey_json       JSONB        NOT NULL,
  invalidated_at_utc TIMESTAMP    DEFAULT NULL
);"#;

        pub const INSERT_ONE: &str = r#"INSERT INTO
    public.passkeys(
        created_at_utc,
        passkey_name,
        credential_id_hex,
        passkey_json
    )
VALUES(
    $1,
    $2,
    $3,
    $4
);"#;

        pub const SELECT_ONE_BY_CREDENTIAL_ID: &str = r#"SELECT
    created_at_utc,
    invalidated_at_utc,
    credential_id_hex,
    passkey_name,
    passkey_json
FROM
    public.passkeys
WHERE
    credential_id_hex = $1;"#;
    }
}

pub fn to_hex_string(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}
