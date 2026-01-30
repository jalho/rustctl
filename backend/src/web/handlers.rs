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

pub async fn auth_init(
    axum::extract::State(state): axum::extract::State<crate::web::State>,
) -> axum::response::Json<crate::web::passkey::RegistrationOptions> {
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

    let options: crate::web::passkey::RegistrationOptions =
        crate::web::passkey::RegistrationOptions {
            challenge: challenge_hex,
            rp: crate::web::passkey::Rp {
                name: "PLACEHOLDER1".into(),
                id: crate::web::DOMAIN_NAME.into(),
            },
            user: crate::web::passkey::User {
                id: "PLACEHOLDER2".into(),
                name: "PLACEHOLDER3".into(),
                display_name: "PLACEHOLDER4".into(),
            },
            pub_key_cred_params: vec![crate::web::passkey::PubKeyCredParam {
                alg: -7, // "ECDSA using P-256 and SHA-256"
                kind: String::from("public-key"),
            }],
            timeout: 60000, // milliseconds
        };

    axum::response::Json(options)
}
