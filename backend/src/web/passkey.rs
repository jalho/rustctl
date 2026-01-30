#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationOptions {
    pub challenge: String,
    pub rp: Rp,
    pub user: User,
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    pub timeout: u64,
}

#[derive(serde::Serialize)]
pub struct Rp {
    pub name: String,
    pub id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

#[derive(serde::Serialize)]
pub struct PubKeyCredParam {
    pub alg: i32,
    #[serde(rename = "type")]
    pub kind: String,
}
