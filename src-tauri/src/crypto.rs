use crate::error::{CryptoError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use keyring::Entry;
use once_cell::sync::OnceCell;
use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM,
};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};

const SERVICE_NAME: &str = "velo-hub";
const KEY_NAME: &str = "master-encryption-key";
const NONCE_LEN: usize = 12;
static MASTER_KEY_CACHE: OnceCell<[u8; 32]> = OnceCell::new();

/// A nonce sequence that generates a random nonce for each encryption operation
struct RandomNonceSequence {
    rng: SystemRandom,
}

impl RandomNonceSequence {
    fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }
}

impl NonceSequence for RandomNonceSequence {
    fn advance(&mut self) -> std::result::Result<Nonce, Unspecified> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes).map_err(|_| Unspecified)?;
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

/// CryptoService handles encryption and decryption of sensitive data using AES-256-GCM
/// The master key is stored securely in the system keychain
pub struct CryptoService {
    master_key: [u8; 32],
    rng: SystemRandom,
}

impl CryptoService {
    /// Create a new CryptoService instance
    /// Retrieves or generates the master key from system keychain
    pub fn new() -> Result<Self> {
        let master_key = Self::get_or_create_master_key()?;
        Ok(Self {
            master_key,
            rng: SystemRandom::new(),
        })
    }

    /// Get or create the master encryption key from system keychain
    fn get_or_create_master_key() -> Result<[u8; 32]> {
        let key = MASTER_KEY_CACHE.get_or_try_init(|| Self::load_master_key_from_keychain())?;

        Ok(*key)
    }

    fn load_master_key_from_keychain() -> std::result::Result<[u8; 32], CryptoError> {
        let entry = Entry::new(SERVICE_NAME, KEY_NAME)
            .map_err(|e| CryptoError::KeyringError(format!("Failed to access keychain: {}", e)))?;

        // Try to retrieve existing key
        match entry.get_password() {
            Ok(key_base64) => {
                // Decode existing key
                let key_bytes = BASE64.decode(key_base64).map_err(|e| {
                    CryptoError::InvalidKeyFormat(format!("Failed to decode master key: {}", e))
                })?;

                if key_bytes.len() != 32 {
                    return Err(CryptoError::InvalidKeyFormat(
                        "Invalid master key length in keychain".to_string(),
                    ));
                }

                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                Ok(key)
            }
            Err(_) => {
                // Generate new key
                let mut key = [0u8; 32];
                let rng = SystemRandom::new();
                rng.fill(&mut key).map_err(|_| {
                    CryptoError::KeyGenerationFailed("Failed to generate random key".to_string())
                })?;

                // Store in keychain
                let key_base64 = BASE64.encode(key);
                entry.set_password(&key_base64).map_err(|e| {
                    CryptoError::KeyringError(format!("Failed to store key in keychain: {}", e))
                })?;

                tracing::info!("Generated and stored new master encryption key in system keychain");
                Ok(key)
            }
        }
    }

    /// Encrypt plaintext using AES-256-GCM
    /// Returns: nonce (12 bytes) + ciphertext + tag (16 bytes)
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>> {
        // Generate nonce
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|_| CryptoError::EncryptionFailed("Failed to generate nonce".to_string()))?;

        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| CryptoError::EncryptionFailed("Failed to create nonce".to_string()))?;

        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.master_key).map_err(|_| {
            CryptoError::EncryptionFailed("Failed to create encryption key".to_string())
        })?;

        let mut sealing_key = SealingKey::new(unbound_key, SingleNonceSequence::new(nonce));

        // Prepare data for encryption
        let mut in_out = plaintext.as_bytes().to_vec();

        // Seal (encrypt) the data
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut in_out)
            .map_err(|_| CryptoError::EncryptionFailed("Encryption failed".to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&in_out);

        Ok(result)
    }

    /// Decrypt ciphertext using AES-256-GCM
    /// Input format: nonce (12 bytes) + ciphertext + tag (16 bytes)
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<String> {
        if ciphertext.len() < NONCE_LEN + 16 {
            return Err(CryptoError::DecryptionFailed("Ciphertext too short".to_string()).into());
        }

        // Extract nonce
        let nonce_bytes = &ciphertext[..NONCE_LEN];
        let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|_| CryptoError::DecryptionFailed("Invalid nonce".to_string()))?;

        // Extract encrypted data
        let encrypted_data = &ciphertext[NONCE_LEN..];

        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.master_key).map_err(|_| {
            CryptoError::DecryptionFailed("Failed to create decryption key".to_string())
        })?;

        let mut opening_key = OpeningKey::new(unbound_key, SingleNonceSequence::new(nonce));

        // Prepare data for decryption
        let mut in_out = encrypted_data.to_vec();

        // Open (decrypt) the data
        let decrypted = opening_key
            .open_in_place(Aad::empty(), &mut in_out)
            .map_err(|_| CryptoError::DecryptionFailed("Decryption failed".to_string()))?;

        // Convert to string
        String::from_utf8(decrypted.to_vec()).map_err(|e| {
            CryptoError::DecryptionFailed(format!("Invalid UTF-8 in decrypted data: {}", e)).into()
        })
    }

    /// Encrypt and encode to base64 for storage
    pub fn encrypt_to_base64(&self, plaintext: &str) -> Result<String> {
        let encrypted = self.encrypt(plaintext)?;
        Ok(BASE64.encode(encrypted))
    }

    /// Decode from base64 and decrypt
    pub fn decrypt_from_base64(&self, base64_ciphertext: &str) -> Result<String> {
        let ciphertext = BASE64.decode(base64_ciphertext).map_err(|e| {
            CryptoError::DecryptionFailed(format!("Failed to decode base64: {}", e))
        })?;
        self.decrypt(&ciphertext)
    }
}

/// A nonce sequence that returns a single nonce once
struct SingleNonceSequence {
    nonce: Option<Nonce>,
}

impl SingleNonceSequence {
    fn new(nonce: Nonce) -> Self {
        Self { nonce: Some(nonce) }
    }
}

impl NonceSequence for SingleNonceSequence {
    fn advance(&mut self) -> std::result::Result<Nonce, Unspecified> {
        self.nonce.take().ok_or(Unspecified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test crypto service with a fixed key
    fn create_test_crypto() -> CryptoService {
        let master_key = [42u8; 32]; // Fixed test key
        CryptoService {
            master_key,
            rng: SystemRandom::new(),
        }
    }

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = create_test_crypto();
        let plaintext = "test-api-key-12345";

        let encrypted = crypto.encrypt(plaintext).expect("Encryption failed");
        assert!(encrypted.len() > plaintext.len());

        let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_base64() {
        let crypto = create_test_crypto();
        let plaintext = "sk-1234567890abcdef";

        let encrypted_base64 = crypto
            .encrypt_to_base64(plaintext)
            .expect("Encryption failed");
        assert!(!encrypted_base64.is_empty());

        let decrypted = crypto
            .decrypt_from_base64(&encrypted_base64)
            .expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let crypto = create_test_crypto();
        let invalid_data = vec![0u8; 10];

        let result = crypto.decrypt(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_encryptions_different_ciphertext() {
        let crypto = create_test_crypto();
        let plaintext = "test-data";

        let encrypted1 = crypto.encrypt(plaintext).expect("Encryption 1 failed");
        let encrypted2 = crypto.encrypt(plaintext).expect("Encryption 2 failed");

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        let decrypted1 = crypto.decrypt(&encrypted1).expect("Decryption 1 failed");
        let decrypted2 = crypto.decrypt(&encrypted2).expect("Decryption 2 failed");
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }
}
