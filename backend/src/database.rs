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

    pub async fn select_one_passkey_by_credential_id(
        &mut self,
        credential_id: &webauthn_rs::prelude::CredentialID,
    ) -> Option<webauthn_rs::prelude::Passkey> {
        log::debug!(
            "Checking credential ID against {count} known passkeys",
            count = self.in_mem.len(),
        );
        for v in self.in_mem.iter() {
            let deserialized: webauthn_rs::prelude::Passkey = serde_json::from_str(v).unwrap();
            let id: &webauthn_rs::prelude::CredentialID = deserialized.cred_id();
            if id == credential_id {
                return Some(deserialized);
            } else {
                log::debug!(
                    "Credential ID {credential_id:?} does NOT match passkey known with credential ID {id:?}: {deserialized:?}"
                );
            }
        }
        None
    }
}
