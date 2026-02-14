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
    state.tx.send(crate::ctl::Command::Reboot).await.unwrap();

    let payload: Vec<u8> = Vec::new();
    let body: axum::body::Body = payload.into();
    axum::response::Response::new(body)
}

pub async fn restart_web_server(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> impl axum::response::IntoResponse {
    state
        .tx
        .send(crate::ctl::Command::RestartWebServer)
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

    let _auth_result: webauthn_rs::prelude::AuthenticationResult = match state
        .webauthn
        .finish_discoverable_authentication(&payload, da.inner, &[passkey_active.into()])
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::UNAUTHORIZED,
    };

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

    /*
     * TODO: Set-Cookie.
     */
    log::info!(
        r#"[{transaction_id}] Sign-in: Existing passkey "{passkey_name}": {credential_id_hex}"#,
        transaction_id = &id.to_string()[..8],
        credential_id_hex = &credential_id_hex[..12],
        passkey_name = passkey_known.passkey_name,
    );
    axum::http::StatusCode::NO_CONTENT
}

const MAX_PENDING: usize = 64;
