use super::paths;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::sync::OnceLock;

static CIPHER: OnceLock<Aes256Gcm> = OnceLock::new();

fn get_cipher() -> &'static Aes256Gcm {
    CIPHER.get_or_init(|| {
        let key_path = paths::get_data_dir().join("secret.key");
        let key_bytes = if key_path.exists() {
             fs::read(&key_path).expect("Failed to read secret key")
        } else {
             let key = Aes256Gcm::generate_key(&mut OsRng);
             fs::write(&key_path, &key).expect("Failed to save secret key");
             key.to_vec()
        };

        if key_bytes.len() != 32 {
            panic!("Invalid key length in secret.key");
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        Aes256Gcm::new(key)
    })
}

pub fn encrypt(data: &str) -> Result<String> {
    let cipher = get_cipher();
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(combined))
}

pub fn decrypt(encrypted_data: &str) -> Result<String> {
    let cipher = get_cipher();
    let decoded = general_purpose::STANDARD
        .decode(encrypted_data)
        .context("Base64 decode failed")?;

    if decoded.len() < 12 {
        return Err(anyhow::anyhow!("Data too short"));
    }

    let (nonce_bytes, ciphertext) = decoded.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    Ok(String::from_utf8(plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let text = "Hello World! Secret";
        let encrypted = encrypt(text).unwrap();
        assert_ne!(text, encrypted);
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(text, decrypted);
    }
}
