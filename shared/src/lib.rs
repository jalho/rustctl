pub const SIGN_UP_CHALLENGE: &str = "/auth/sign-up/challenge";
pub const SIGN_UP_SUBMIT: &str = "/auth/sign-up/submit/{challenge_id}";

pub const SIGN_IN_CHALLENGE: &str = "/auth/sign-in/challenge";
pub const SIGN_IN_SUBMIT: &str = "/auth/sign-in/submit";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignUpResponse {
    pub id: uuid::Uuid,
    pub ccr: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignInResponse {
    pub id: uuid::Uuid,
    pub rcr: serde_json::Value,
}
