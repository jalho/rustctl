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
}
