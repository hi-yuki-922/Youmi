//! keyring 后端：Windows 凭据管理器（ADR-0001 凭据存储 seam）。
//!
//! 平台实现不做单测：行为由 `#[ignore]` smoke 测试覆盖（会写真实凭据库）。

use keyring::Entry;

use crate::platform::{PlatformError, SecretStore};

/// 凭据条目的服务名：Youmi 全部秘密共用一个服务，按 key 区分条目。
const SERVICE: &str = "youmi";

pub struct WinSecretStore;

impl SecretStore for WinSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, PlatformError> {
        match entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(PlatformError(format!("读取凭据 {key} 失败: {e}"))),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        entry(key)?
            .set_password(value)
            .map_err(|e| PlatformError(format!("写入凭据 {key} 失败: {e}")))
    }

    fn delete(&self, key: &str) -> Result<(), PlatformError> {
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(PlatformError(format!("删除凭据 {key} 失败: {e}"))),
        }
    }
}

fn entry(key: &str) -> Result<Entry, PlatformError> {
    Entry::new(SERVICE, key).map_err(|e| PlatformError(format!("凭据条目 {key} 初始化失败: {e}")))
}

#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    #[ignore = "写真实系统凭据库，手动执行：cargo test -- --ignored --test-threads=1"]
    fn set_get_delete_roundtrip() {
        let key = format!("youmi-smoke-{}", std::process::id());
        let store = WinSecretStore;
        assert_eq!(store.get(&key).unwrap(), None);
        store.set(&key, "secret-value").unwrap();
        assert_eq!(store.get(&key).unwrap(), Some("secret-value".into()));
        store.set(&key, "secret-value-2").unwrap();
        assert_eq!(store.get(&key).unwrap(), Some("secret-value-2".into()));
        store.delete(&key).unwrap();
        // 重复删除与删除后读取均为"不存在"语义
        store.delete(&key).unwrap();
        assert_eq!(store.get(&key).unwrap(), None);
    }
}
