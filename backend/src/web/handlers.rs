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
) -> axum::response::Json<serde_json::Value> {
    let mut challenge_bytes: [u8; 32] = [0u8; 32];
    {
        let mut generator: rand::prelude::ThreadRng = rand::rng();
        use rand::RngCore;
        generator.fill_bytes(&mut challenge_bytes);
    }

    let challenge_hex: String = challenge_bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    /*
     * TODO: Revoke pending transaction after some timeout.
     */
    let pending_count: usize;
    {
        let mut lock = state.pending_challenges.lock().await;
        lock.insert(challenge_hex.clone());
        pending_count = lock.len();
    }
    log::debug!("Auth transactions pending: {pending_count}");

    axum::response::Json(serde_json::json!({
        "challenge": challenge_hex,
        "rp": {
            "name": "PLACEHOLDER1",
            "id": crate::web::DOMAIN_NAME,
        },
        "user": {
            "id": "PLACEHOLDER2",
            "name": "PLACEHOLDER3",
            "displayName": "PLACEHOLDER4",
        },
        "pubKeyCredParams": [{
            "alg": -7,
            "type": "public-key",
        }],
        "timeout": 60000,
    }))
}

pub async fn auth_sign_up_submit(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    log::debug!("Inbound Sign-Up Credential: {:#?}", payload);
    axum::http::StatusCode::NO_CONTENT
}

pub async fn auth_sign_in_challenge(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Json<serde_json::Value> {
    let mut challenge_bytes: [u8; 32] = [0u8; 32];
    {
        let mut generator: rand::prelude::ThreadRng = rand::rng();
        use rand::RngCore;
        generator.fill_bytes(&mut challenge_bytes);
    }

    let challenge_hex: String = challenge_bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    /*
     * TODO: Revoke pending transaction after some timeout.
     */
    let pending_count: usize;
    {
        let mut lock = state.pending_challenges.lock().await;
        lock.insert(challenge_hex.clone());
        pending_count = lock.len();
    }
    log::debug!("Auth transactions pending: {pending_count}");

    axum::response::Json(serde_json::json!({
        "challenge": challenge_hex,
        "rpId": crate::web::DOMAIN_NAME,
        "timeout": 60000,
        "userVerification": "required"
    }))
}

pub async fn auth_sign_in_submit(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    log::debug!("Inbound Sign-In Credential: {:#?}", payload);
    axum::http::StatusCode::NO_CONTENT
}
