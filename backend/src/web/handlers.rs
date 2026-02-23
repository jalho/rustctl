pub async fn favicon() -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .unwrap()
}

pub async fn web() -> impl axum::response::IntoResponse {
    /*
     * Cheatsheet:
     *
     * ```sh
     * ln -s $(pwd)/frontend/index.html /var/lib/rustctl/www/index.html
     * ```
     */
    let path = if cfg!(debug_assertions) {
        "frontend/index.html"
    } else {
        "/var/lib/rustctl/www/index.html"
    };

    match tokio::fs::read_to_string(path).await {
        Ok(mut html) => {
            html = html
                .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
                .replace("{{SIGN_UP_CHALLENGE}}", shared::SIGN_UP_CHALLENGE)
                .replace("{{SIGN_UP_SUBMIT}}", shared::SIGN_UP_SUBMIT)
                .replace("{{SIGN_IN_CHALLENGE}}", shared::SIGN_IN_CHALLENGE)
                .replace("{{SIGN_IN_SUBMIT}}", shared::SIGN_IN_SUBMIT);

            axum::response::Response::builder()
                .header("content-type", "text/html")
                .body(axum::body::Body::from(html))
                .unwrap()
        }
        Err(err) => {
            log::error!(
                "Failed to read frontend at {}: {}",
                path,
                crate::get_full_error_message(&err),
            );

            axum::response::Response::builder()
                .status(axum::http::StatusCode::NOT_IMPLEMENTED)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }
}

pub async fn reboot_system(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Response {
    state
        .tx
        .send(crate::ctl::CommandFromWebClient::Reboot)
        .await
        .unwrap();

    let payload: Vec<u8> = Vec::new();
    let body: axum::body::Body = payload.into();
    axum::response::Response::new(body)
}

pub async fn restart_web_server(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> impl axum::response::IntoResponse {
    state
        .tx
        .send(crate::ctl::CommandFromWebClient::RestartWebServer)
        .await
        .unwrap();

    axum::http::StatusCode::NO_CONTENT
}

async fn prune_pending<T>(
    pending_map: &tokio::sync::Mutex<
        std::collections::HashMap<uuid::Uuid, crate::web::Timestamped<T>>,
    >,
    transaction_type_for_log: &str,
) where
    T: std::fmt::Debug + Clone,
{
    let mut removable: Vec<uuid::Uuid> = Vec::new();

    {
        let lock = pending_map.lock().await;
        if lock.len() >= MAX_PENDING {
            let mut ordered: Vec<(&uuid::Uuid, &crate::web::Timestamped<T>)> =
                lock.iter().collect();

            /*
             * Select some of the oldest for removal.
             */
            ordered.sort_by_key(|n| n.1.timestamp);
            'select_for_removal: for (k, _v) in ordered {
                removable.push(*k);
                if removable.len() >= MAX_PENDING / 2 {
                    break 'select_for_removal;
                }
            }
        }
    }

    let removable_count: usize = removable.len();
    if removable_count > 0 {
        {
            let mut lock = pending_map.lock().await;
            for k in removable {
                lock.remove(&k);
            }
        }
        log::warn!(
            "Removed {removable_count} pending {transaction_type_for_log} transactions from memory"
        );
    }
}

pub async fn auth_sign_up_challenge(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<shared::SignUpRequest>,
) -> axum::response::Json<shared::SignUpResponse> {
    let created_at: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    prune_pending(&state.pending_signups, "sign-up").await;

    let passkey_name: String = payload.passkey_name.to_string();

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let (ccr, pkr) = state
        .webauthn
        .start_passkey_registration(id, &passkey_name, &passkey_name, None)
        .expect("Failed to start registration.");
    let ccr: webauthn_rs::prelude::CreationChallengeResponse = ccr;
    let pkr: webauthn_rs::prelude::PasskeyRegistration = pkr;

    let timestamped: crate::web::Timestamped<crate::web::NamedPasskeyRegistration> =
        crate::web::Timestamped::new(
            &created_at,
            crate::web::NamedPasskeyRegistration::new(&passkey_name, pkr),
        );

    let pending_count: usize;
    {
        let mut lock = state.pending_signups.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }

    let ccr_serializable: serde_json::Value = serde_json::to_value(&ccr).unwrap();

    log::info!(
        "[{transaction_id}] New sign-up initiated: Pending sign-ups in total: {pending_count}",
        transaction_id = &id.to_string()[..8],
    );
    shared::SignUpResponse {
        id,
        ccr: ccr_serializable,
    }
    .into()
}

pub async fn auth_sign_up_submit(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(mut state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    let pending: Option<crate::web::Timestamped<crate::web::NamedPasskeyRegistration>>;
    {
        let mut lock = state.pending_signups.lock().await;
        pending = lock.remove(&id);
    }

    let named_pkr: crate::web::Timestamped<crate::web::NamedPasskeyRegistration> = match pending {
        Some(n) => n,
        None => return axum::http::StatusCode::BAD_REQUEST,
    };

    let rpkc: webauthn_rs::prelude::RegisterPublicKeyCredential =
        match serde_json::from_value(payload) {
            Ok(cred) => cred,
            Err(_err) => return axum::http::StatusCode::BAD_REQUEST,
        };

    let passkey: webauthn_rs::prelude::Passkey = match state
        .webauthn
        .finish_passkey_registration(&rpkc, &named_pkr.inner.pkr)
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::BAD_REQUEST,
    };

    let created_at: chrono::DateTime<chrono::Utc> = named_pkr.timestamp;
    if state
        .db_client
        .insert_one_passkey(crate::database::queries::PasskeyInsertable {
            created_at,
            passkey_name: named_pkr.inner.passkey_name.to_owned(),
            passkey: passkey.clone(),
        })
        .await
        .is_err()
    {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    /*
     * TODO: Set-Cookie.
     */
    log::info!(
        r#"[{transaction_id}] Sign-up: New passkey "{passkey_name}": {credential_id_hex}"#,
        transaction_id = &id.to_string()[..8],
        credential_id_hex = &crate::database::to_hex_string(passkey.cred_id())[..12],
        passkey_name = named_pkr.inner.passkey_name,
    );
    axum::http::StatusCode::NO_CONTENT
}

pub async fn auth_sign_in_challenge(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Json<shared::SignInResponse> {
    let init_at: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    prune_pending(&state.pending_signins, "sign-in").await;

    let (rcr, da) = state.webauthn.start_discoverable_authentication().unwrap();
    let rcr: webauthn_rs::prelude::RequestChallengeResponse = rcr;
    let da: webauthn_rs::prelude::DiscoverableAuthentication = da;

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let timestamped: crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication> =
        crate::web::Timestamped::new(&init_at, da);

    let pending_count: usize;
    {
        let mut lock = state.pending_signins.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }

    let rcr_serializable: serde_json::Value = serde_json::to_value(&rcr).unwrap();

    log::info!(
        "[{transaction_id}] New sign-in initiated: Pending sign-ins in total: {pending_count}",
        transaction_id = &id.to_string()[..8],
    );
    shared::SignInResponse {
        id,
        rcr: rcr_serializable,
    }
    .into()
}

pub async fn auth_sign_in_submit(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(mut state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<webauthn_rs::prelude::PublicKeyCredential>,
) -> axum::http::StatusCode {
    let pending: Option<crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication>>;
    {
        let mut lock = state.pending_signins.lock().await;
        pending = lock.remove(&id);
    }

    let da: crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication> =
        match pending {
            Some(n) => n,
            None => return axum::http::StatusCode::BAD_REQUEST,
        };

    let (c_pk_id, c_pk_cred_id) = match state
        .webauthn
        .identify_discoverable_authentication(&payload)
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::BAD_REQUEST,
    };
    let _claimed_passkey_id: uuid::Uuid = c_pk_id;
    let claimed_passkey_cred_id: &[u8] = c_pk_cred_id;

    let passkey_seeked: Option<crate::database::queries::PasskeySelected> = match state
        .db_client
        .select_one_passkey_by_credential_id(claimed_passkey_cred_id)
        .await
    {
        Ok(n) => n,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    let passkey_known: crate::database::queries::PasskeySelected = match passkey_seeked {
        Some(n) => n,
        None => return axum::http::StatusCode::UNAUTHORIZED,
    };
    let passkey_active: webauthn_rs::prelude::Passkey = match passkey_known.invalidated_at {
        Some(_) => return axum::http::StatusCode::UNAUTHORIZED,
        None => passkey_known.passkey,
    };
    let credential_id_hex: String = crate::database::to_hex_string(passkey_active.cred_id());

    let auth_result: webauthn_rs::prelude::AuthenticationResult = match state
        .webauthn
        .finish_discoverable_authentication(&payload, da.inner, &[passkey_active.into()])
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::UNAUTHORIZED,
    };
    let credential_counter_new: u32 = auth_result.counter();

    /*
     * TODO: From `finish_discoverable_authentication`:
     *
     * > As per <https://www.w3.org/TR/webauthn-3/#sctn-verifying-assertion> 21:
     * >
     * > If the Credential Counter is greater than 0 you MUST assert that the
     * > counter is greater than the stored counter. If the counter is equal
     * > or less than this MAY indicate a cloned credential and you SHOULD
     * > invalidate and reject that credential as a result.
     * >
     * > From this [AuthenticationResult] you *should* update the Credential's
     * > Counter value if it is valid per the above check. If you wish you *may*
     * > use the content of the [AuthenticationResult] for extended validations
     * > (such as the user verification flag).
     */

    if (state
        .db_client
        .update_one_passkey_by_credential_id_set_credential_counter(
            claimed_passkey_cred_id,
            credential_counter_new,
        )
        .await)
        .is_err()
    {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    /*
     * TODO: Set-Cookie.
     */
    log::info!(
        r#"[{transaction_id}] Sign-in: Existing passkey "{passkey_name}": {credential_id_hex} (#{credential_counter_new})"#,
        transaction_id = &id.to_string()[..8],
        credential_id_hex = &credential_id_hex[..12],
        passkey_name = passkey_known.passkey_name,
    );
    axum::http::StatusCode::NO_CONTENT
}

const MAX_PENDING: usize = 64;

pub async fn poc_require_cookie_signed(
    _session_verified: Session,
) -> impl axum::response::IntoResponse {
    axum::http::StatusCode::NO_CONTENT
}

pub async fn poc_set_cookie_signed(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> impl axum::response::IntoResponse {
    let timestamp: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    let session: Session = Session {
        issued_at: timestamp.to_rfc3339(),
        steam_id: None,
    };

    let session_json: String = serde_json::to_string(&session).unwrap();
    let session_hex: String = crate::database::to_hex_string(session_json.as_bytes());

    let mut signing_randomness: [u8; libcrux_ml_dsa::SIGNING_RANDOMNESS_SIZE] =
        [0u8; libcrux_ml_dsa::SIGNING_RANDOMNESS_SIZE];
    rand::TryRng::try_fill_bytes(&mut rand::rngs::SysRng, &mut signing_randomness).unwrap();

    /*
     * ML-DSA-87: NIST security level 5, why not, let's go :D
     */
    let signature_obj: libcrux_ml_dsa::MLDSASignature<_> = libcrux_ml_dsa::ml_dsa_87::sign(
        &state.signing_keypair.signing_key,
        session_hex.as_bytes(),
        b"",
        signing_randomness,
    )
    .unwrap();

    let signature_hex: String = crate::database::to_hex_string(signature_obj.as_slice());

    let mut response_builder =
        axum::response::Response::builder().status(axum::http::StatusCode::NO_CONTENT);

    let ck_session: String = format!("{CK_NAME_SESSION}={session_hex}; {CK_ATTRS}");
    response_builder = response_builder.header(axum::http::header::SET_COOKIE, ck_session);

    let signature_chars: Vec<char> = signature_hex.chars().collect();
    for (i, chunk) in signature_chars.chunks(2048).enumerate() {
        let chunk_str: String = chunk.iter().collect();
        let ck_signature: String = format!("{CK_NAME_SIG}-{i}={chunk_str}; {CK_ATTRS}");
        response_builder = response_builder.header(axum::http::header::SET_COOKIE, ck_signature);
    }

    response_builder.body(axum::body::Body::empty()).unwrap()
}

const CK_ATTRS: &str = "Path=/; HttpOnly; SameSite=Strict; Secure";
const CK_NAME_SESSION: &str = "rustctl-session-hex";
const CK_NAME_SIG: &str = "rustctl-signature-hex";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Session {
    issued_at: String,
    steam_id: Option<String>,
}

impl axum::extract::FromRequestParts<crate::web::State> for Session {
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &crate::web::State,
    ) -> Result<Self, Self::Rejection> {
        /*
         * Extract cookies header.
         */
        let cookie_header: &str = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

        /*
         * Extract session and its signature from the cookies.
         */
        let mut session_hex_maybe: Option<&str> = None;
        let mut signature_chunks: std::collections::BTreeMap<usize, &str> =
            std::collections::BTreeMap::new();

        for cookie in cookie_header.split(';') {
            let mut kv = cookie.splitn(2, '=');
            let k: Option<&str> = kv.next().map(|s| s.trim());
            let v: Option<&str> = kv.next().map(|s| s.trim());

            if let (Some(key), Some(val)) = (k, v) {
                if key == CK_NAME_SESSION {
                    session_hex_maybe = Some(val);
                } else if key.starts_with(CK_NAME_SIG)
                    && let Some(index_str) = key
                        .strip_prefix(CK_NAME_SIG)
                        .and_then(|s| s.strip_prefix('-'))
                    && let Ok(index) = index_str.parse::<usize>()
                {
                    signature_chunks.insert(index, val);
                }
            }
        }

        let session_hex = session_hex_maybe.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

        if signature_chunks.is_empty() {
            return Err(axum::http::StatusCode::UNAUTHORIZED);
        }
        let signature_hex: String = signature_chunks.values().cloned().collect();

        /*
         * Decode signature from hex.
         */
        let mut sig_bytes: [u8; super::SIGNATURE_SIZE_BYTES] = [0u8; super::SIGNATURE_SIZE_BYTES];
        if signature_hex.len() != super::SIGNATURE_SIZE_BYTES * 2 {
            return Err(axum::http::StatusCode::UNAUTHORIZED);
        }
        for i in (0..signature_hex.len()).step_by(2) {
            let byte_hex: &str = &signature_hex[i..i + 2];
            let byte: u8 = match u8::from_str_radix(byte_hex, 16) {
                Ok(n) => n,
                Err(_) => return Err(axum::http::StatusCode::UNAUTHORIZED),
            };
            sig_bytes[i / 2] = byte;
        }
        let signature: libcrux_ml_dsa::ml_dsa_87::MLDSA87Signature =
            libcrux_ml_dsa::ml_dsa_87::MLDSA87Signature::new(sig_bytes);

        /*
         * Verify signature.
         */
        libcrux_ml_dsa::ml_dsa_87::verify(
            &state.signing_keypair.verification_key,
            session_hex.as_bytes(),
            b"",
            &signature,
        )
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

        /*
         * Deserialize the hex encoded JSON session.
         */
        let mut session_bytes: Vec<u8> = Vec::with_capacity(session_hex.len() / 2);
        for i in (0..session_hex.len()).step_by(2) {
            let byte_hex: &str = &session_hex[i..i + 2];
            let byte: u8 = match u8::from_str_radix(byte_hex, 16) {
                Ok(n) => n,
                Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
            };
            session_bytes.push(byte);
        }
        let session: Session = serde_json::from_slice(&session_bytes)
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(session)
    }
}
