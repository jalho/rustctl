trait DecodeHex {
    fn to_decoded(&self) -> Result<Vec<u8>, String>;
}

impl DecodeHex for &str {
    fn to_decoded(&self) -> Result<Vec<u8>, String> {
        self.trim()
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                let hex_byte: &str = match std::str::from_utf8(chunk) {
                    Ok(n) => n,
                    Err(err) => return Err(err.to_string()),
                };
                match u8::from_str_radix(hex_byte, 16) {
                    Ok(n) => Ok(n),
                    Err(err) => Err(err.to_string()),
                }
            })
            .collect()
    }
}

#[test]
fn from_vdf_steamcmd_contaminated() {
    let hex: &'static str = include_str!("./sample-001.hex");
    let decoded: Vec<u8> = hex.to_decoded().unwrap();
    let utf8: String = String::from_utf8(decoded).unwrap();
    let _buildid = crate::steam::BuildID::from_vdf_steamcmd_contaminated(&utf8).unwrap();
}
