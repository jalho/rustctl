#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EnvTime(pub f64);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PlayerPos {
    pub steam_id: String,
    pub display_name: String,
    pub position: (f64, f64, f64),
    pub rotation: (f64, f64, f64),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Player {
    #[serde(rename = "SteamID")]
    pub steam_id: String,
    #[serde(rename = "OwnerSteamID")]
    pub owner_steam_id: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "Ping")]
    pub ping: i32,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "EntityId")]
    pub entity_id: i32,
    #[serde(rename = "ConnectedSeconds")]
    pub connected_seconds: i32,
    #[serde(rename = "ViolationLevel")]
    pub violation_level: f64,
    #[serde(rename = "CurrentLevel")]
    pub current_level: f64,
    #[serde(rename = "UnspentXp")]
    pub unspent_xp: f64,
    #[serde(rename = "Health")]
    pub health: f64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Toolcupboard {
    pub entity_id: i32,
    pub position: (f64, f64, f64),
    pub auth_count: u32,
}
