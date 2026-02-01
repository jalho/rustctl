pub const SIGN_UP_CHALLENGE: &str = "/auth/sign-up/challenge";
pub const SIGN_UP_SUBMIT: &str = "/auth/sign-up/submit/{challenge_id}";

pub const SIGN_IN_CHALLENGE: &str = "/auth/sign-in/challenge";
pub const SIGN_IN_SUBMIT: &str = "/auth/sign-in/submit/{challenge_id}";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignUpRequest {
    /// Some user defined name for the passkey.
    pub passkey_name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignUpResponse {
    /// Ephemeral transaction ID.
    pub id: uuid::Uuid,

    /// Named after `webauthn_rs::prelude::CreationChallengeResponse` in
    /// `webauthn_rs` version 0.5.4.
    pub ccr: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignInResponse {
    /// Ephemeral transaction ID.
    pub id: uuid::Uuid,

    /// Named after `webauthn_rs::prelude::RequestChallengeResponse` in
    /// `webauthn_rs` version 0.5.4.
    pub rcr: serde_json::Value,
}
