//! 密码本加解密：Argon2id 派生密钥 + AES-256-GCM（ADR-0002）。
//!
//! 密文 blob 格式为 `nonce(12B) || ciphertext+tag`，nonce 每次加密随机生成；
//! 盐值独立存储（`vault_meta.salt`），同一主密码全程只用一份派生密钥。
//! 错误主密码的识别完全依赖 GCM 认证标签：解密失败即密码错误（或密文损坏，
//! 对外同为一个错误，不区分二者）。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use zeroize::Zeroizing;

/// Argon2id 盐值长度（字节）。
pub const SALT_LEN: usize = 16;
/// 派生密钥长度（字节），即 AES-256 的密钥长度。
pub const KEY_LEN: usize = 32;
/// AES-GCM nonce 长度（字节）。
const NONCE_LEN: usize = 12;
/// 密文 blob 最短长度：nonce + GCM tag（16B）。
const MIN_BLOB_LEN: usize = NONCE_LEN + 16;

/// 解密校验器用的已知明文：解锁时能还原出它即证明主密码正确。
pub const VERIFIER_PLAINTEXT: &[u8] = b"youmi-vault-check-v1";

/// 加解密失败。错误信息面向用户展示，不泄露内部细节。
#[derive(Debug)]
pub struct CryptoError(pub String);

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CryptoError {}

/// 生成随机盐值。
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    salt
}

/// 主密码 + 盐值 → 256 位密钥（Argon2id，库默认参数）。
pub fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>, CryptoError> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|e| CryptoError(format!("密钥派生失败: {e}")))?;
    Ok(key)
}

/// 加密为 `nonce || ciphertext+tag` blob。
pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|e| CryptoError(format!("加密失败: {e}")))?;
    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// 解密 `nonce || ciphertext+tag` blob；密钥不对或 blob 损坏统一报错。
pub fn open(key: &[u8; KEY_LEN], blob: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if blob.len() < MIN_BLOB_LEN {
        return Err(CryptoError("密文数据不完整".into()));
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(key.into());
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| CryptoError("解密失败：主密码错误或数据损坏".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn seal_and_open_roundtrip() {
        let salt = generate_salt();
        let key = derive_key(PASSWORD, &salt).unwrap();
        let plaintext = "标题/用户名/密码/URL/备注 🎉".as_bytes();

        let blob = seal(&key, plaintext).unwrap();
        assert_ne!(blob, plaintext.to_vec());
        let recovered = open(&key, &blob).unwrap();
        assert_eq!(recovered, plaintext.to_vec());
    }

    #[test]
    fn wrong_password_fails_to_open() {
        let salt = generate_salt();
        let key = derive_key(PASSWORD, &salt).unwrap();
        let blob = seal(&key, b"secret").unwrap();

        let wrong = derive_key("wrong-password", &salt).unwrap();
        let err = open(&wrong, &blob).unwrap_err();
        assert!(err.0.contains("主密码错误"));
    }

    #[test]
    fn different_salt_yields_different_key() {
        // 同一主密码配不同盐：彼此解不开对方的密文
        let key_a = derive_key(PASSWORD, &generate_salt()).unwrap();
        let key_b = derive_key(PASSWORD, &generate_salt()).unwrap();
        let blob = seal(&key_a, b"secret").unwrap();
        assert!(open(&key_b, &blob).is_err());

        // 同盐同密码：派生结果确定，可解
        let salt = generate_salt();
        let key_1 = derive_key(PASSWORD, &salt).unwrap();
        let key_2 = derive_key(PASSWORD, &salt).unwrap();
        assert_eq!(*key_1, *key_2);
    }

    #[test]
    fn each_seal_uses_fresh_nonce() {
        let key = derive_key(PASSWORD, &generate_salt()).unwrap();
        let blob_a = seal(&key, b"same plaintext").unwrap();
        let blob_b = seal(&key, b"same plaintext").unwrap();
        assert_ne!(blob_a, blob_b);
        // 两个 blob 各自都能解开
        assert_eq!(open(&key, &blob_a).unwrap(), b"same plaintext".to_vec());
        assert_eq!(open(&key, &blob_b).unwrap(), b"same plaintext".to_vec());
    }

    #[test]
    fn tampered_ciphertext_fails_to_open() {
        let key = derive_key(PASSWORD, &generate_salt()).unwrap();
        let mut blob = seal(&key, b"secret").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        assert!(open(&key, &blob).is_err());
    }

    #[test]
    fn truncated_or_tiny_blob_is_rejected() {
        let key = derive_key(PASSWORD, &generate_salt()).unwrap();
        assert!(open(&key, &[]).is_err());
        assert!(open(&key, &[1, 2, 3]).is_err());
    }
}
