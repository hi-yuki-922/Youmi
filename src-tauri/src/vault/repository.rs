//! 密码本 Repository：SQLite 之上的密文存取深模块。
//!
//! 只搬运字节，不懂密钥：盐值/校验器/条目密文的原样存取，加解密在
//! `crypto`（密钥侧）与会话状态（编排侧）。外部行为（也是测试断言的全部）：
//! - `initialize` / `meta`：主密码盐值与校验器的单行存取
//! - `entries`：条目列表，`updated_at` 降序，不含解密
//! - `insert_entry` / `update_entry` / `delete_entry`

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::db;

/// 密码本元信息：Argon2id 盐值 + 主密码校验器（AES-GCM 密文 blob）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultMeta {
    pub salt: Vec<u8>,
    pub verifier: Vec<u8>,
}

/// 条目的密文行：载荷 blob（nonce‖密文）原样存取，由会话层加解密。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEntry {
    pub id: i64,
    pub data: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct VaultRepository {
    conn: Mutex<Connection>,
}

impl VaultRepository {
    /// 打开（必要时创建）数据库文件并确保 schema 就绪。
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Ok(Self::from_conn(db::open(path)?))
    }

    /// 内存数据库（测试用）。
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Ok(Self::from_conn(db::open_in_memory()?))
    }

    fn from_conn(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// 是否已设置主密码（meta 行是否存在）。
    pub fn is_initialized(&self) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT 1 FROM vault_meta WHERE id = 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .optional()
        .map(|row| row.is_some())
    }

    /// 写入 meta（盐值 + 校验器）。已初始化时主键冲突报错，避免覆盖既有密文。
    pub fn initialize(&self, meta: &VaultMeta) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vault_meta (id, salt, verifier) VALUES (1, ?1, ?2)",
            params![meta.salt, meta.verifier],
        )?;
        Ok(())
    }

    pub fn meta(&self) -> rusqlite::Result<Option<VaultMeta>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT salt, verifier FROM vault_meta WHERE id = 1",
            [],
            |row| {
                Ok(VaultMeta {
                    salt: row.get(0)?,
                    verifier: row.get(1)?,
                })
            },
        )
        .optional()
    }

    /// 全部条目（密文行），`updated_at` 降序。
    pub fn entries(&self) -> rusqlite::Result<Vec<StoredEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, data, created_at, updated_at FROM vault_entries ORDER BY updated_at DESC, id DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(StoredEntry {
                id: row.get(0)?,
                data: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// 取单条密文行。
    pub fn entry(&self, id: i64) -> rusqlite::Result<Option<StoredEntry>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, data, created_at, updated_at FROM vault_entries WHERE id = ?1",
            params![id],
            |row| {
                Ok(StoredEntry {
                    id: row.get(0)?,
                    data: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .optional()
    }

    /// 新增条目，返回 id。
    pub fn insert_entry(&self, data: &[u8]) -> rusqlite::Result<i64> {
        let now = now_ms();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vault_entries (data, created_at, updated_at) VALUES (?1, ?2, ?2)",
            params![data, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新条目载荷（updated_at 刷新）；返回是否确有更新。
    pub fn update_entry(&self, id: i64, data: &[u8]) -> rusqlite::Result<bool> {
        let now = now_ms();
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE vault_entries SET data = ?1, updated_at = ?2 WHERE id = ?3",
            params![data, now, id],
        )?;
        Ok(updated > 0)
    }

    /// 删除条目；返回是否确有删除。
    pub fn delete_entry(&self, id: i64) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM vault_entries WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn repo() -> VaultRepository {
        VaultRepository::open_in_memory().unwrap()
    }

    fn meta(salt_byte: u8) -> VaultMeta {
        VaultMeta {
            salt: vec![salt_byte; 16],
            verifier: vec![salt_byte + 1; 40],
        }
    }

    #[test]
    fn fresh_vault_is_not_initialized() {
        let repo = repo();
        assert!(!repo.is_initialized().unwrap());
        assert_eq!(repo.meta().unwrap(), None);
    }

    #[test]
    fn initialize_then_meta_roundtrip() {
        let repo = repo();
        let m = meta(0xAA);
        repo.initialize(&m).unwrap();

        assert!(repo.is_initialized().unwrap());
        assert_eq!(repo.meta().unwrap(), Some(m));
    }

    #[test]
    fn double_initialize_is_rejected() {
        let repo = repo();
        repo.initialize(&meta(1)).unwrap();
        assert!(repo.initialize(&meta(2)).is_err());
        // 首次初始化的数据不被覆盖
        assert_eq!(repo.meta().unwrap(), Some(meta(1)));
    }

    #[test]
    fn entry_crud_roundtrip() {
        let repo = repo();
        let id = repo.insert_entry(&[1, 2, 3]).unwrap();

        let entry = repo.entry(id).unwrap().unwrap();
        assert_eq!(entry.data, vec![1, 2, 3]);

        assert!(repo.update_entry(id, &[9, 9]).unwrap());
        assert_eq!(repo.entry(id).unwrap().unwrap().data, vec![9, 9]);

        assert!(repo.delete_entry(id).unwrap());
        assert_eq!(repo.entry(id).unwrap(), None);
        assert!(!repo.delete_entry(id).unwrap());
        assert!(!repo.update_entry(id, &[1]).unwrap());
    }

    #[test]
    fn entries_ordered_newest_update_first() {
        let repo = repo();
        let a = repo.insert_entry(&[1]).unwrap();
        let b = repo.insert_entry(&[2]).unwrap();
        // 人为把 a 的 updated_at 抬到最新，验证排序跟随 updated_at 而非插入序
        repo.update_entry(a, &[1, 1]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        repo.update_entry(b, &[2, 2]).unwrap();

        let entries = repo.entries().unwrap();
        assert_eq!(entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![b, a]);
    }

    #[test]
    fn vault_persists_across_reopen() {
        let dir = TempDir::new("vault-repo-test");
        let path = dir.path().join("youmi.db");

        {
            let repo = VaultRepository::open(&path).unwrap();
            repo.initialize(&meta(7)).unwrap();
            repo.insert_entry(&[4, 5, 6]).unwrap();
        }
        {
            let repo = VaultRepository::open(&path).unwrap();
            assert!(repo.is_initialized().unwrap());
            assert_eq!(repo.meta().unwrap(), Some(meta(7)));
            assert_eq!(repo.entries().unwrap().len(), 1);
        }
    }
}
