#[allow(clippy::large_enum_variant)]
pub enum DbOp {
    InsertOnePasskey {
        tx: tokio::sync::oneshot::Sender<CredentialID>,
        value: queries::PasskeyInsertable,
    },
    SelectOnePasskeyByCredentialId {
        tx: tokio::sync::oneshot::Sender<Option<queries::PasskeySelected>>,
        selector: CredentialID,
    },
    UpdateOnePasskeyByCredentialIdSetCredentialCounter {
        tx: tokio::sync::oneshot::Sender<()>,
        selector: CredentialID,
        value: u32,
    },

    InsertOneTlsPem {
        tx: tokio::sync::oneshot::Sender<()>,
        value: queries::TlsPemInsertable,
    },
    SelectOneTlsPemLatest {
        tx: tokio::sync::oneshot::Sender<Option<queries::TlsPemSelected>>,
    },
}

pub mod queries {
    pub struct PasskeyInsertable {
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub passkey_name: String,
        pub passkey: webauthn_rs::prelude::Passkey,
    }

    pub struct PasskeySelected {
        pub invalidated_at: Option<chrono::DateTime<chrono::Utc>>,
        pub passkey_name: String,
        pub passkey: webauthn_rs::prelude::Passkey,
    }

    pub struct TlsPemInsertable {
        pub private_key_pem: String,
        pub certificate_pem: String,
    }

    pub struct TlsPemSelected {
        pub private_key_pem: String,
        pub certificate_pem: String,
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

    pub async fn insert_one_tls_pem(&mut self, value: queries::TlsPemInsertable) -> Result<(), ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self.tx.send(DbOp::InsertOneTlsPem { tx, value }).await {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub async fn select_tls_pem_latest(&mut self) -> Result<Option<queries::TlsPemSelected>, ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self.tx.send(DbOp::SelectOneTlsPemLatest { tx }).await {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(n) => Ok(n),
            Err(_) => Err(()),
        }
    }

    pub async fn insert_one_passkey(
        &mut self,
        value: queries::PasskeyInsertable,
    ) -> Result<CredentialID, ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self.tx.send(DbOp::InsertOnePasskey { tx, value }).await {
            Ok(_) => {}
            Err(_) => todo!(),
        }

        match rx.await {
            Ok(n) => Ok(n),
            Err(_) => Err(()),
        }
    }

    pub async fn select_one_passkey_by_credential_id(
        &mut self,
        selector: &CredentialID,
    ) -> Result<Option<queries::PasskeySelected>, ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::SelectOnePasskeyByCredentialId {
                tx,
                selector: selector.clone(),
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

    pub async fn update_one_passkey_by_credential_id_set_credential_counter(
        &mut self,
        selector: &CredentialID,
        value: u32,
    ) -> Result<(), ()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        match self
            .tx
            .send(DbOp::UpdateOnePasskeyByCredentialIdSetCredentialCounter {
                tx,
                selector: selector.clone(),
                value,
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
        let mut retry_delay_secs: u64 = 1;
        'reconnect: loop {
            let params: Parameters = Parameters::from_env();

            let (mut client, connection): (
                tokio_postgres::Client,
                tokio_postgres::Connection<
                    tokio_postgres::Socket,
                    tokio_postgres::tls::NoTlsStream,
                >,
            ) = match tokio_postgres::connect(
                &format!(
                    "postgresql://{user}:{password}@127.0.0.1:5432/{database}?connect_timeout=1",
                    user = params.user,
                    password = params.password,
                    database = params.database,
                ),
                tokio_postgres::NoTls,
            )
            .await
            {
                Ok(n) => {
                    retry_delay_secs = 1;
                    n
                }
                Err(err) => {
                    log::error!("{err}", err = crate::get_full_error_message(&err));
                    tokio::time::sleep(std::time::Duration::from_secs(retry_delay_secs)).await;
                    if retry_delay_secs < 32 {
                        retry_delay_secs *= 2;
                    }
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
                log::info!("Connected to existing database");
            }

            tokio::select!(
                _ = self.handle_queries(client) => {}
                _ = connection_handle => {}
            )
        }
    }

    async fn assure_tables_exist(client: &mut tokio_postgres::Client) -> bool {
        let mut some_tables_created = false;

        match client.execute(tables::passkeys::CREATE_TABLE, &[]).await {
            Ok(_) => some_tables_created = true,
            Err(err) => {
                let msg: String = crate::get_full_error_message(&err);
                if !msg.contains("already exists") {
                    panic!("{msg}");
                }
            }
        }

        match client.execute(tables::tls_pem::CREATE_TABLE, &[]).await {
            Ok(_) => some_tables_created = true,
            Err(err) => {
                let msg: String = crate::get_full_error_message(&err);
                if !msg.contains("already exists") {
                    panic!("{msg}");
                }
            }
        }

        some_tables_created
    }

    async fn handle_queries(&mut self, client: tokio_postgres::Client) {
        while let Some(n) = self.rx.recv().await {
            match n {
                DbOp::InsertOnePasskey { tx, value } => {
                    let created_at_utc: chrono::NaiveDateTime = value.created_at.naive_utc();

                    let credential_id: CredentialID = CredentialID::new(value.passkey.cred_id());
                    let credential_id_hex: String = credential_id.to_string();

                    let passkey_json: serde_json::Value =
                        serde_json::to_value(&value.passkey).unwrap();

                    let credential_counter: i64 = 0;

                    let inserted_count: u64 = match client
                        .execute(
                            tables::passkeys::INSERT_ONE,
                            &[
                                &created_at_utc,
                                &credential_counter,
                                &value.passkey_name,
                                &credential_id_hex,
                                &passkey_json,
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

                    match tx.send(credential_id) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }

                DbOp::SelectOnePasskeyByCredentialId { tx, selector } => {
                    let rows: Vec<tokio_postgres::Row> = match client
                        .query(
                            tables::passkeys::SELECT_ONE_BY_CREDENTIAL_ID,
                            &[&selector.to_string()],
                        )
                        .await
                    {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("{err}", err = crate::get_full_error_message(&err));
                            return;
                        }
                    };

                    let found: Option<queries::PasskeySelected> = match rows.len() {
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

                            Some(queries::PasskeySelected {
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

                DbOp::UpdateOnePasskeyByCredentialIdSetCredentialCounter {
                    tx,
                    selector,
                    value,
                } => {
                    let value: i64 = value as i64;

                    let modified_count: u64 = match client
                        .execute(
                            tables::passkeys::UPDATE_ONE_BY_CREDENTIAL_ID_SET_CREDENTIAL_COUNTER,
                            &[&selector.to_string(), &value],
                        )
                        .await
                    {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("{err}", err = crate::get_full_error_message(&err));
                            return;
                        }
                    };
                    assert_eq!(modified_count, 1);

                    tx.send(()).unwrap();
                }

                DbOp::InsertOneTlsPem { tx, value } => {
                    let cert_decoded: openssl::x509::X509 =
                        openssl::x509::X509::from_pem(value.certificate_pem.as_bytes()).unwrap();

                    let serial_number: &openssl::asn1::Asn1IntegerRef =
                        cert_decoded.serial_number();

                    let serial_number_hex_x509: String = into_colon_delimited_hex_lower_case(
                        &serial_number.to_bn().unwrap().to_vec(),
                    );

                    let not_before_utc: chrono::NaiveDateTime =
                        crate::web::asn1_to_chrono(cert_decoded.not_before()).naive_utc();

                    let not_after_utc: chrono::NaiveDateTime =
                        crate::web::asn1_to_chrono(cert_decoded.not_after()).naive_utc();

                    let inserted_count: u64 = match client
                        .execute(
                            tables::tls_pem::INSERT_ONE,
                            &[
                                &serial_number_hex_x509,
                                &not_before_utc,
                                &not_after_utc,
                                &value.private_key_pem,
                                &value.certificate_pem,
                            ],
                        )
                        .await
                    {
                        Ok(n) => n,
                        Err(_) => todo!(),
                    };
                    assert_eq!(inserted_count, 1);

                    match tx.send(()) {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }

                DbOp::SelectOneTlsPemLatest { tx } => {
                    let rows: Vec<tokio_postgres::Row> =
                        match client.query(tables::tls_pem::SELECT_ONE_LATEST, &[]).await {
                            Ok(n) => n,
                            Err(_) => todo!(),
                        };

                    let found: Option<queries::TlsPemSelected> = match rows.len() {
                        0 => None,
                        1 => {
                            let row: &tokio_postgres::Row = rows.first().unwrap();

                            let private_key_pem: String = {
                                let deserialized = row.try_get("private_key_pem");
                                let value: String = match deserialized {
                                    Ok(n) => n,
                                    Err(_) => todo!(),
                                };
                                value
                            };

                            let certificate_pem: String = {
                                let deserialized = row.try_get("certificate_pem");
                                let value: String = match deserialized {
                                    Ok(n) => n,
                                    Err(_) => todo!(),
                                };
                                value
                            };

                            Some(queries::TlsPemSelected {
                                private_key_pem,
                                certificate_pem,
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
        pub const CREATE_TABLE: &str = r#"CREATE TABLE rustctl.passkeys (
  created_at_utc     TIMESTAMP    NOT NULL,
  credential_counter INT8         NOT NULL,
  passkey_name       TEXT         NOT NULL,
  credential_id_hex  TEXT         PRIMARY KEY,
  passkey_json       JSONB        NOT NULL,
  invalidated_at_utc TIMESTAMP    DEFAULT NULL
);"#;

        pub const INSERT_ONE: &str = r#"INSERT INTO
    rustctl.passkeys(
        created_at_utc,
        credential_counter,
        passkey_name,
        credential_id_hex,
        passkey_json
    )
VALUES(
    $1,
    $2,
    $3,
    $4,
    $5
);"#;

        pub const SELECT_ONE_BY_CREDENTIAL_ID: &str = r#"SELECT
    created_at_utc,
    invalidated_at_utc,
    credential_id_hex,
    passkey_name,
    passkey_json
FROM
    rustctl.passkeys
WHERE
    credential_id_hex = $1;"#;

        pub const UPDATE_ONE_BY_CREDENTIAL_ID_SET_CREDENTIAL_COUNTER: &str = r#"UPDATE
    rustctl.passkeys
SET
    credential_counter = $2
WHERE
    credential_id_hex = $1;"#;
    }

    pub mod tls_pem {
        pub const CREATE_TABLE: &str = r#"CREATE TABLE rustctl.tls_pem (
  serial_number_hex_x509  TEXT      PRIMARY KEY,
  not_before_utc          TIMESTAMP NOT NULL,
  not_after_utc           TIMESTAMP NOT NULL,
  private_key_pem         TEXT      NOT NULL,
  certificate_pem         TEXT      NOT NULL
);"#;

        pub const INSERT_ONE: &str = r#"INSERT INTO
    rustctl.tls_pem(
        serial_number_hex_x509,
        not_before_utc,
        not_after_utc,
        private_key_pem,
        certificate_pem
    )
VALUES(
    $1,
    $2,
    $3,
    $4,
    $5
);"#;

        pub const SELECT_ONE_LATEST: &str = r#"SELECT
    serial_number_hex_x509,
    not_before_utc,
    not_after_utc,
    private_key_pem,
    certificate_pem
FROM
    rustctl.tls_pem
ORDER BY
    not_before_utc DESC
LIMIT 1;"#;
    }
}

struct Parameters {
    user: String,
    password: String,
    database: String,
}

impl Parameters {
    pub fn from_env() -> Self {
        const VAR_NAME: &str = "POSTGRES_PASSWORD";
        let password: String = match std::env::var(VAR_NAME) {
            Ok(n) => n,
            Err(err) => {
                panic!(
                    "Missing required env var {VAR_NAME}: {err}",
                    err = crate::get_full_error_message(&err),
                );
            }
        };
        Self {
            user: "rustctl".into(),
            password,
            database: "rustctl".into(),
        }
    }
}

#[derive(Clone)]
pub struct CredentialID(Vec<u8>);

impl CredentialID {
    pub fn new(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl std::fmt::Display for CredentialID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: String = into_colon_delimited_hex_lower_case(&self.0);
        write!(f, "{value}")
    }
}

impl serde::Serialize for CredentialID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for CredentialID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let bytes = s
            .split(':')
            .map(|chunk| u8::from_str_radix(chunk, 16).map_err(serde::de::Error::custom))
            .collect::<Result<Vec<u8>, D::Error>>()?;

        Ok(CredentialID(bytes))
    }
}

/// Example:
///
/// ```
/// "2a:28:e9:b6:46:c7:a6:8a:db:76:ee:5f:6c:04:00:7b:dc:e3:ca:0f"
/// ```
fn into_colon_delimited_hex_lower_case(buf: &[u8]) -> String {
    buf.to_vec()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join(":")
}
