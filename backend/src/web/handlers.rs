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

/// Sample as of commit `0396d82b9c6dbe03ab4fd5a61b99738b0438254b`:
///
/// ```
/// Object {
///     "authenticatorAttachment": String("platform"),
///     "clientExtensionResults": Object {},
///     "id": String("mbTdK6Kai4XyMER2M8Hw9FYBeaD2en_9R3FP3THvja4"),
///     "rawId": String("mbTdK6Kai4XyMER2M8Hw9FYBeaD2en_9R3FP3THvja4"),
///     "response": Object {
///         "attestationObject": String("o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YVikaaXLHvXupl3n6pt9CYu3T3VGjjezzsTRn8ZN6K_saDxFAAAAAAiYcFjK3EuBtuEw3lDcvpYAIJm03SuimouF8jBEdjPB8PRWAXmg9np__UdxT90x742upQECAyYgASFYIGOko1Z5h85mJxafTuVYTYoPuFxbSQ_Z_kJV70_kBzwUIlggIL2y5GSB7pOT4FeMl_YoD7J3wOHyT0RsUpTW48IhAo4"),
///         "authenticatorData": String("aaXLHvXupl3n6pt9CYu3T3VGjjezzsTRn8ZN6K_saDxFAAAAAAiYcFjK3EuBtuEw3lDcvpYAIJm03SuimouF8jBEdjPB8PRWAXmg9np__UdxT90x742upQECAyYgASFYIGOko1Z5h85mJxafTuVYTYoPuFxbSQ_Z_kJV70_kBzwUIlggIL2y5GSB7pOT4FeMl_YoD7J3wOHyT0RsUpTW48IhAo4"),
///         "clientDataJSON": String("eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoicWRUbmVZdmJJRjNsWTdtU3dkanYzcl9GbU5tRmwxakpvaDJzUmFrcFNpbyIsIm9yaWdpbiI6Imh0dHBzOi8vcnVzdGN0bC5pbnRlcm5hbDo4MDgwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ"),
///         "publicKey": String("MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEY6SjVnmHzmYnFp9O5VhNig-4XFtJD9n-QlXvT-QHPBQgvbLkZIHuk5PgV4yX9igPsnfA4fJPRGxSlNbjwiECjg"),
///         "publicKeyAlgorithm": Number(-7),
///         "transports": Array [
///             String("internal"),
///         ],
///     },
///     "type": String("public-key"),
/// }
/// ```
pub async fn auth_sign_up_submit(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    /*
     * TODO: Verify and respond with a Set-Cookie.
     */
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

/// Sample as of commit `0396d82b9c6dbe03ab4fd5a61b99738b0438254b`:
///
/// ```
/// Object {
///     "authenticatorAttachment": String("platform"),
///     "clientExtensionResults": Object {},
///     "id": String("mbTdK6Kai4XyMER2M8Hw9FYBeaD2en_9R3FP3THvja4"),
///     "rawId": String("mbTdK6Kai4XyMER2M8Hw9FYBeaD2en_9R3FP3THvja4"),
///     "response": Object {
///         "authenticatorData": String("aaXLHvXupl3n6pt9CYu3T3VGjjezzsTRn8ZN6K_saDwFAAAAAQ"),
///         "clientDataJSON": String("eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiRXRkeTc4U2F2eU1fYWNUcjMxMElfZzN6dmhEMHYxQ3BBM0RNLWs4V0k5YyIsIm9yaWdpbiI6Imh0dHBzOi8vcnVzdGN0bC5pbnRlcm5hbDo4MDgwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlLCJvdGhlcl9rZXlzX2Nhbl9iZV9hZGRlZF9oZXJlIjoiZG8gbm90IGNvbXBhcmUgY2xpZW50RGF0YUpTT04gYWdhaW5zdCBhIHRlbXBsYXRlLiBTZWUgaHR0cHM6Ly9nb28uZ2wveWFiUGV4In0"),
///         "signature": String("MEUCIQCFBnBIg0UjZ9B2r8W9d6xZG85VOMaiVI8Iq3atG8ByfwIgKflayU5ybwtVu-8y0AOkK0VoyeaYnxtpc_p0BrvPYAk"),
///         "userHandle": String("UExBQ0VIT0xERVIy"),
///     },
///     "type": String("public-key"),
/// }
/// ```
pub async fn auth_sign_in_submit(
    axum::extract::State(_state): axum::extract::State<crate::web::State>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> axum::http::StatusCode {
    /*
     * TODO: Verify and respond with a Set-Cookie.
     */
    log::debug!("Inbound Sign-In Credential: {:#?}", payload);
    axum::http::StatusCode::NO_CONTENT
}
