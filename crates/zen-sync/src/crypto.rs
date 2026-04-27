// Placeholder for E2EE sync encryption.
// Architecture: per-account root key → per-device wrapped key → per-record AEAD envelope.

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid or missing key")]
    InvalidKey,
}

pub struct SyncEncryptor {
    // TODO: hold derived device key material
}

impl SyncEncryptor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt(&self, _plaintext: &[u8]) -> Vec<u8> {
        // TODO: AES-256-GCM or ChaCha20-Poly1305 envelope encryption
        Vec::new()
    }

    pub fn decrypt(&self, _ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // TODO: AEAD decryption + authentication tag verification
        Ok(Vec::new())
    }
}

impl Default for SyncEncryptor {
    fn default() -> Self {
        Self::new()
    }
}
