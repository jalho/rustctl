pub async fn favicon() -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .unwrap()
}

pub async fn web() -> impl axum::response::IntoResponse {
    let path = "/var/lib/rustctl/www/index.html";
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
        Err(_) => axum::response::Response::builder()
            .status(axum::http::StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("Frontend not found"))
            .unwrap(),
    }
}

pub async fn reboot(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Response {
    state.tx.send(crate::ctl::Command::Reboot).await.unwrap();

    let payload: Vec<u8> = Vec::new();
    let body: axum::body::Body = payload.into();
    axum::response::Response::new(body)
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
    prune_pending(&state.pending_signups, "sign-up").await;

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let (ccr, pkr) = state
        .webauthn
        .start_passkey_registration(id, &payload.passkey_name, &payload.passkey_name, None)
        .expect("Failed to start registration.");
    let ccr: webauthn_rs::prelude::CreationChallengeResponse = ccr;
    let pkr: webauthn_rs::prelude::PasskeyRegistration = pkr;

    let timestamped: crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration> =
        crate::web::Timestamped::new(pkr);

    let pending_count: usize;
    {
        let mut lock = state.pending_signups.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }
    log::debug!("New sign-up initiated: Pending sign-up transactions in total: {pending_count}");

    let ccr_serializable: serde_json::Value = serde_json::to_value(&ccr).unwrap();

    shared::SignUpResponse {
        id,
        ccr: ccr_serializable,
    }
    .into()
}

pub async fn auth_sign_up_submit(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    let pending_count: usize;
    let pending: Option<crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration>>;
    {
        let mut lock = state.pending_signups.lock().await;
        pending = lock.remove(&id);
        pending_count = lock.len();
    }

    if pending.is_some() {
        // count changed
        log::debug!(
            "Sign-up submitted: Pending sign-up transactions remaining in total: {pending_count}"
        );
    }

    let pkr: crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration> = match pending {
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
        .finish_passkey_registration(&rpkc, &pkr.inner)
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::BAD_REQUEST,
    };

    let passkeys_registered_globally: usize;
    {
        let mut lock = state.db.lock().await;
        lock.insert_one_passkey(&passkey).await;
        passkeys_registered_globally = lock.select_all_passkeys().await.len();
    }
    log::debug!(
        "Registered 1 new passkey with credential ID {cred_id:?}: Passkeys registered globally in total: {passkeys_registered_globally}",
        cred_id = passkey.cred_id(),
    );

    /*
     * TODO: Set-Cookie.
     */
    axum::http::StatusCode::NO_CONTENT
}

pub async fn auth_sign_in_challenge(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Json<shared::SignInResponse> {
    prune_pending(&state.pending_signins, "sign-in").await;

    let (rcr, da) = state.webauthn.start_discoverable_authentication().unwrap();
    let rcr: webauthn_rs::prelude::RequestChallengeResponse = rcr;
    let da: webauthn_rs::prelude::DiscoverableAuthentication = da;

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let timestamped: crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication> =
        crate::web::Timestamped::new(da);

    let pending_count: usize;
    {
        let mut lock = state.pending_signins.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }
    log::debug!("New sign-in initiated: Pending sign-in transactions in total: {pending_count}");

    let rcr_serializable: serde_json::Value = serde_json::to_value(&rcr).unwrap();
    shared::SignInResponse {
        id,
        rcr: rcr_serializable,
    }
    .into()
}

pub async fn auth_sign_in_submit(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    axum::extract::State(state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<webauthn_rs::prelude::PublicKeyCredential>,
) -> axum::http::StatusCode {
    let pending_count: usize;
    let pending: Option<crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication>>;
    {
        let mut lock = state.pending_signins.lock().await;
        pending = lock.remove(&id);
        pending_count = lock.len();
    }

    let da: crate::web::Timestamped<webauthn_rs::prelude::DiscoverableAuthentication> =
        match pending {
            Some(n) => {
                // count changed
                log::debug!(
                    "Sign-in submitted: Pending sign-in transactions remaining in total: {pending_count}"
                );
                n
            }
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

    let passkey_seeked: Option<webauthn_rs::prelude::Passkey>;
    {
        let mut lock = state.db.lock().await;
        passkey_seeked = lock
            .select_one_passkey_by_credential_id(claimed_passkey_cred_id)
            .await;
    }
    let passkey_known: webauthn_rs::prelude::Passkey = match passkey_seeked {
        Some(n) => n,
        None => return axum::http::StatusCode::UNAUTHORIZED,
    };
    let passkey_known: webauthn_rs::prelude::DiscoverableKey = passkey_known.into();

    let _auth_result: webauthn_rs::prelude::AuthenticationResult = match state
        .webauthn
        .finish_discoverable_authentication(&payload, da.inner, &[passkey_known])
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
    axum::http::StatusCode::NO_CONTENT
}

const MAX_PENDING: usize = 64;
