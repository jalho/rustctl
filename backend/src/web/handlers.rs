pub async fn favicon() -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .unwrap()
}

pub async fn web() -> impl axum::response::IntoResponse {
    let path = "/home/rustctl/rustctl/target/dx/frontend/release/web/public/index.html";
    match tokio::fs::read_to_string(path).await {
        Ok(html) => axum::response::Response::builder()
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .unwrap(),
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

pub async fn auth_sign_up_challenge(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Json<shared::SignUpResponse> {
    /*
     * Make some space if there are too many pending in memory.
     */
    let mut removable: Vec<uuid::Uuid> = Vec::new();
    {
        let lock = state.pending_signups.lock().await;
        if lock.len() >= MAX_PENDING {
            let mut ordered: Vec<(
                &uuid::Uuid,
                &crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration>,
            )> = lock.iter().collect();

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
            let mut lock = state.pending_signups.lock().await;
            while let Some(k) = removable.pop() {
                lock.remove(&k);
            }
        }
        log::warn!("Removed {removable_count} pending sign-up transactions from memory");
    }

    /*
     * Store new pending in memory.
     */
    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let (ccr, pkr) = state
        .webauthn
        .start_passkey_registration(id, &id.to_string(), &id.to_string(), None)
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
    log::debug!("Pending sign-up transactions in total: {pending_count}");

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
        log::debug!("Pending sing-up transactions in total: {pending_count}");
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
        "Passkeys registered globally: {passkeys_registered_globally} (added 1 with credential ID {cred_id:?})",
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
    /*
     * Make some space if there are too many pending in memory.
     */
    let mut removable: Vec<uuid::Uuid> = Vec::new();
    {
        let lock = state.pending_signins.lock().await;
        if lock.len() >= MAX_PENDING {
            let mut ordered: Vec<(
                &uuid::Uuid,
                &crate::web::Timestamped<webauthn_rs::prelude::PasskeyAuthentication>,
            )> = lock.iter().collect();

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
            let mut lock = state.pending_signins.lock().await;
            while let Some(k) = removable.pop() {
                lock.remove(&k);
            }
        }
        log::warn!("Removed {removable_count} pending sign-in transactions from memory");
    }

    /*
     * "Username-less" login, i.e. let the client discover passkeys (associated
     * with the domain) and we just accept any. We'll then associate passkeys
     * with stuff when the passkey holding user does things.
     *
     * For example, a passkey holder may later link a Steam account. More
     * passkeys can be associated with the same Steam account later by anyone
     * who has the ability to do so, and so the Steam account becomes the
     * thing that identifies a user that may have multiple passkeys across e.g.
     * multiple devices.
     *
     * This implies that the system may have "anonymous users", i.e. those with
     * a passkey installed for the domain but no further associations.
     */
    let no_keys = [
        // None!
    ];
    let (rcr, pka) = state
        .webauthn
        .start_passkey_authentication(&no_keys)
        .unwrap();
    let rcr: webauthn_rs::prelude::RequestChallengeResponse = rcr;
    let pka: webauthn_rs::prelude::PasskeyAuthentication = pka;

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let timestamped: crate::web::Timestamped<webauthn_rs::prelude::PasskeyAuthentication> =
        crate::web::Timestamped::new(pka);

    let pending_count: usize;
    {
        let mut lock = state.pending_signins.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }
    log::debug!("Pending sign-in transactions in total: {pending_count}");

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
    let pending: Option<crate::web::Timestamped<webauthn_rs::prelude::PasskeyAuthentication>>;
    {
        let mut lock = state.pending_signins.lock().await;
        pending = lock.remove(&id);
        pending_count = lock.len();
    }

    let pka: crate::web::Timestamped<webauthn_rs::prelude::PasskeyAuthentication> = match pending {
        Some(n) => {
            // count changed
            log::debug!("Pending sign-in transactions in total: {pending_count}");
            n
        }
        None => return axum::http::StatusCode::BAD_REQUEST,
    };

    let auth_result: webauthn_rs::prelude::AuthenticationResult = match state
        .webauthn
        .finish_passkey_authentication(&payload, &pka.inner)
    {
        Ok(n) => n,
        Err(_err) => return axum::http::StatusCode::BAD_REQUEST,
    };

    /*
     * Assert that the authenticated passkey (of a user who is potentially still
     * anonymous) exists in the database, to be sure that it was registered
     * for this system. Otherwise it's only bound to the domain, I guess, whose
     * control could change over time.
     */
    let existing: Option<webauthn_rs::prelude::Passkey>;
    let claimed: &webauthn_rs::prelude::CredentialID = auth_result.cred_id();
    {
        let mut lock = state.db.lock().await;
        existing = lock.select_one_passkey_by_credential_id(claimed).await;
    }
    if existing.is_none() {
        return axum::http::StatusCode::UNAUTHORIZED;
    }

    /*
     * TODO: Make sense of this:
     *
     * > As per https://www.w3.org/TR/webauthn-3/#sctn-verifying-assertion 21:
     * >
     * > If the Credential Counter is greater than 0 you MUST assert that the counter is greater than the stored counter. If the counter is equal or less than this MAY indicate a cloned credential and you SHOULD invalidate and reject that credential as a result.
     * >
     * > From this AuthenticationResult you should update the Credential’s Counter value if it is valid per the above check. If you wish you may use the content of the AuthenticationResult for extended validations (such as the presence of the user verification flag).
     *
     * From:
     * https://docs.rs/webauthn-rs/0.5.4/webauthn_rs/struct.Webauthn.html#method.finish_passkey_authentication
     * (accessed 2026-01-31)
     */

    /*
     * TODO: Set-Cookie.
     */
    axum::http::StatusCode::NO_CONTENT
}

const MAX_PENDING: usize = 64;
