//! Domain-separated signatures; module catalog signatures cannot be replayed.
use ed25519_dalek::{Signature, VerifyingKey};
pub const CONTEXT: &[u8] = b"freeRGB effect catalog v1\0";
pub fn verify_with_keys(
    bytes: &[u8],
    signature: &[u8],
    keys: &[VerifyingKey],
) -> Result<(), String> {
    let text = std::str::from_utf8(signature).map_err(|e| e.to_string())?;
    let signature = hex::decode(text.trim()).map_err(|e| e.to_string())?;
    let signature = Signature::from_slice(&signature).map_err(|e| e.to_string())?;
    let mut message = CONTEXT.to_vec();
    message.extend_from_slice(bytes);
    if keys
        .iter()
        .any(|key| key.verify_strict(&message, &signature).is_ok())
    {
        Ok(())
    } else {
        Err("Effect catalog signature is not trusted".into())
    }
}
pub fn verify(bytes: &[u8], signature: &[u8]) -> Result<(), String> {
    let keys = include_str!("../keys/release-keys.txt")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| {
            let key: [u8; 32] = hex::decode(s)
                .map_err(|e| e.to_string())?
                .try_into()
                .map_err(|_| "Invalid effect release key")?;
            VerifyingKey::from_bytes(&key).map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    verify_with_keys(bytes, signature, &keys)
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proof {
    pub catalog: String,
    pub signature: String,
}
impl Proof {
    pub fn verify(&self) -> Result<crate::marketplace::Catalog, String> {
        verify(self.catalog.as_bytes(), self.signature.as_bytes())?;
        crate::marketplace::Catalog::parse(self.catalog.as_bytes())
    }
}
