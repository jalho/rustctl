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
        let lock = state.pending.lock().await;
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
            let mut lock = state.pending.lock().await;
            while let Some(k) = removable.pop() {
                lock.remove(&k);
            }
        }
        log::warn!("Removed {removable_count} pending transactions from memory");
    }

    /*
     * Store new pending in memory.
     */
    let rp_id: &str = crate::web::DOMAIN_NAME;
    let rp_origin: url::Url = url::Url::parse(&format!(
        "https://{domain_name}",
        domain_name = crate::web::DOMAIN_NAME,
    ))
    .unwrap();
    let builder: webauthn_rs::WebauthnBuilder<'_> =
        webauthn_rs::WebauthnBuilder::new(rp_id, &rp_origin).unwrap();
    let webauthn: webauthn_rs::Webauthn = builder.build().unwrap();

    let id: uuid::Uuid = uuid::Uuid::new_v4();
    let (ccr, pkr) = webauthn
        .start_passkey_registration(id, "PLACEHOLDER1", "PLACEHOLDER2", None)
        .expect("Failed to start registration.");
    let ccr: webauthn_rs::prelude::CreationChallengeResponse = ccr;
    let pkr: webauthn_rs::prelude::PasskeyRegistration = pkr;

    let timestamped: crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration> =
        crate::web::Timestamped::new(pkr);

    let pending_count: usize;
    {
        let mut lock = state.pending.lock().await;
        lock.insert(id, timestamped);
        pending_count = lock.len();
    }
    log::debug!("Pending transactions in total: {pending_count}");

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
    log::debug!("Inbound Sign-Up Credential: {:#?}", payload);

    let pending_count: usize;
    let pending: Option<crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration>>;
    {
        let mut lock = state.pending.lock().await;
        pending = lock.remove(&id);
        pending_count = lock.len();
    }

    if pending.is_some() {
        // count changed
        log::debug!("Pending transactions in total: {pending_count}");
    }

    let pkr: crate::web::Timestamped<webauthn_rs::prelude::PasskeyRegistration> = match pending {
        Some(n) => n,
        None => return axum::http::StatusCode::BAD_REQUEST,
    };
    log::debug!("Identified pkr: {pkr:?}", pkr = pkr.inner);

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
    log::debug!("{passkey:#?}");

    /*
     * TODO: Store in DB and Set-Cookie.
     */
    axum::http::StatusCode::NO_CONTENT
}

pub async fn auth_sign_in_challenge(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
) -> axum::http::StatusCode {
    /*
     * TODO: Use `start_passkey_authentication`.
     */
    axum::http::StatusCode::NO_CONTENT
}

pub async fn auth_sign_in_submit(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    /*
     * TODO: Verify and respond with a Set-Cookie.
     *
     *       Use `finish_passkey_authentication`.
     */
    log::debug!("Inbound Sign-In Credential: {:#?}", payload);
    axum::http::StatusCode::NO_CONTENT
}

const MAX_PENDING: usize = 64;
