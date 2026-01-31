pub struct Client {
    in_mem: Vec<String>,
}

impl Client {
    pub fn new() -> Self {
        Self { in_mem: Vec::new() }
    }

    pub async fn insert_one_passkey(&mut self, passkey: &webauthn_rs::prelude::Passkey) {
        let serialized: String = serde_json::to_string(passkey).unwrap();
        self.in_mem.push(serialized);
    }

    pub async fn select_all_passkeys(&mut self) -> Vec<webauthn_rs::prelude::Passkey> {
        let mut vec = Vec::new();
        for v in self.in_mem.iter() {
            let deserialized: webauthn_rs::prelude::Passkey = serde_json::from_str(v).unwrap();
            vec.push(deserialized);
        }
        vec
    }
}
