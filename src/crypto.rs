use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngExt;
use sha2::{Digest, Sha256};
use std::error::Error;

/// Derives a 32-byte key from the given PIN/Secret using SHA-256.
pub fn derive_key(secret: &str) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    let result: [u8; 32] = hasher.finalize().into();

    Key::from(result)
}

/// Encrypts a plaintext message using ChaCha20-Poly1305 with the derived key.
/// Returns a base64 encoded string containing the nonce and ciphertext.
pub fn encrypt_message(key: &Key, plaintext: &str) -> Result<String, Box<dyn Error>> {
    let cipher = ChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failure: {}", e))?;

    // Prepend nonce to ciphertext
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(b64.encode(combined))
}

/// Decrypts a base64 encoded message (nonce + ciphertext) using the derived key.
pub fn decrypt_message(key: &Key, b64_message: &str) -> Result<String, Box<dyn Error>> {
    let combined = b64.decode(b64_message)?;
    if combined.len() < 12 {
        return Err("Message too short".into());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce_arr: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| "Invalid nonce length")?;
    let nonce = Nonce::from(nonce_arr);

    let cipher = ChaCha20Poly1305::new(key);
    let plaintext_bytes = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| format!("Decryption failure: {}", e))?;

    Ok(String::from_utf8(plaintext_bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let secret = "nexus-super-secret-pin-123456";
        let key = derive_key(secret);

        let original_msg = "Hello Nexus WebRTC Remote Desktop!";
        let encrypted = encrypt_message(&key, original_msg).unwrap();
        assert_ne!(original_msg, encrypted);

        let decrypted = decrypt_message(&key, &encrypted).unwrap();
        assert_eq!(original_msg, decrypted);
    }

    #[test]
    fn test_invalid_key_fails_decryption() {
        let key1 = derive_key("pin-1");
        let key2 = derive_key("pin-2");

        let original_msg = "Confidential Data";
        let encrypted = encrypt_message(&key1, original_msg).unwrap();

        let result = decrypt_message(&key2, &encrypted);
        assert!(result.is_err());
    }
}
