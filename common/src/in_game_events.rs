#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum DamageType {
    Known(DamageTypeKnown),
    Other(String),
}

#[derive(Debug, serde::Deserialize)]
pub enum DamageTypeKnown {
    Bite,
    Drowned,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Resource {
    Known(ResourceKnown),
    Other(String),
}

#[derive(Debug, serde::Deserialize)]
pub enum ResourceKnown {
    #[serde(rename = "Animal Fat")]
    AnimalFat,
    #[serde(rename = "Blue Berry")]
    BlueBerry,
    Cloth,
    #[serde(rename = "Diesel Fuel")]
    DieselFuel,
    #[serde(rename = "Green Berry")]
    GreenBerry,
    Leather,
    #[serde(rename = "Metal Ore")]
    MetalOre,
    Mushroom,
    #[serde(rename = "Raw Bear Meat")]
    RawBearMeat,
    #[serde(rename = "Red Berry")]
    RedBerry,
    Stones,
    #[serde(rename = "Sulfur Ore")]
    SulfurOre,
    #[serde(rename = "White Berry")]
    WhiteBerry,
    Wood,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "hook")]
pub enum InGameEvent {
    OnDispenserGather {
        steam_id: u64,

        amount: f64,
        resource: Resource,
    },
    OnDispenserBonus {
        steam_id: u64,

        amount: f64,
        resource: Resource,
    },
    OnGrowableGathered {
        steam_id: u64,

        amount: f64,
        resource: Resource,
    },
    OnCollectiblePickup {
        steam_id: u64,

        items: Vec<Item>,
    },
    OnCargoShipSpawnCrate,
    OnPlayerDeath {
        steam_id_killer: Option<u64>,
        steam_id_killed: u64,

        majority_damage_type: Option<DamageType>,
    },
}

#[derive(Debug, serde::Deserialize)]
struct Item {
    resource: Resource,
    amount: f64,
}
