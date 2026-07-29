use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::{Rng, RngExt};
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose::STANDARD as b64};
use std::error::Error;

/// Derives a 32-byte key from the given PIN using SHA-256.
pub fn derive_key_from_pin(pin: &str) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    let result = hasher.finalize();
    
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&result);
    *Key::from_slice(&key_bytes)
}

/// Encrypts a plaintext message using ChaCha20-Poly1305 with the derived key.
/// Returns a base64 encoded string containing the nonce and ciphertext.
pub fn encrypt_message(key: &Key, plaintext: &str) -> Result<String, Box<dyn Error>> {
    let cipher = ChaCha20Poly1305::new(key);
    
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
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
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let cipher = ChaCha20Poly1305::new(key);
    let plaintext_bytes = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failure: {}", e))?;
    
    Ok(String::from_utf8(plaintext_bytes)?)
}
