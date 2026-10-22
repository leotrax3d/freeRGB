//! Trust anchor: Ed25519 public keys compiled into the daemon.
//!
//! Chain: built-in key -> detached catalog signature -> SHA-256 and size of
//! every package in the catalog -> per-file SHA-256 in the installed package
//! manifest, rechecked before each module start. A hash from the download
//! location alone is never trusted.
use crate::{invalid, Result};
use ed25519_dalek::{Signature, VerifyingKey};

/// Domain separation: a catalog signature cannot be replayed for anything else.
pub const CATALOG_CONTEXT: &[u8] = b"freeRGB module catalog v1\0";

const RELEASE_KEYS: &str = include_str!("../keys/release-keys.txt");

#[derive(Clone, Default)]
pub struct TrustedKeys {
    keys: Vec<(VerifyingKey, bool)>,
}

pub fn parse_key(text: &str) -> Result<VerifyingKey> {
    let bytes: [u8; 32] = hex::decode(text.trim())
        .map_err(invalid)?
        .try_into()
        .map_err(|_| invalid("public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&bytes).map_err(invalid)
}

impl TrustedKeys {
    /// Release keys shipped with this build.
    pub fn builtin() -> Self {
        let keys = RELEASE_KEYS
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| parse_key(l).map(|k| (k, false)))
            .collect::<Result<Vec<_>>>()
            .expect("keys/release-keys.txt contains an invalid key");
        Self { keys }
    }

    /// Adds a development key (explicit opt-in, never default).
    pub fn with_development_key(mut self, key: VerifyingKey) -> Self {
        self.keys.push((key, true));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn has_release_key(&self) -> bool {
        self.keys.iter().any(|(_, dev)| !dev)
    }

    /// `signature` is the detached signature file: 128 hex characters.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        if self.keys.is_empty() {
            return Err(invalid(
                "this build has no module signing key; catalog cannot be authenticated",
            ));
        }
        let text = std::str::from_utf8(signature).map_err(invalid)?;
        let bytes: [u8; 64] = hex::decode(text.trim())
            .map_err(invalid)?
            .try_into()
            .map_err(|_| invalid("signature must be 64 bytes"))?;
        let signature = Signature::from_bytes(&bytes);
        let mut signed = CATALOG_CONTEXT.to_vec();
        signed.extend_from_slice(message);
        if self
            .keys
            .iter()
            .any(|(k, _)| k.verify_strict(&signed, &signature).is_ok())
        {
            Ok(())
        } else {
            Err(invalid(
                "catalog signature is not valid for any trusted key",
            ))
        }
    }
}

/// Produces the detached signature text (used by the release tool and tests).
pub fn sign(key: &ed25519_dalek::SigningKey, message: &[u8]) -> String {
    use ed25519_dalek::Signer;
    let mut signed = CATALOG_CONTEXT.to_vec();
    signed.extend_from_slice(message);
    hex::encode(key.sign(&signed).to_bytes())
}
