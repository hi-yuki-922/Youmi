//! SQLite 数据库：连接打开 + migration 机制。
//!
//! 剪贴板历史与密码本密文共用一库；新增 schema 一律在本文件
//! `MIGRATIONS` 末尾追加迁移，禁止改动已有迁移。

use rusqlite::Connection;

/// 迁移 V1：剪贴板历史表。
///
/// - `recency`：单调递增的排序序号（"到顶" = 取 MAX+1），不受系统时钟回拨影响
/// - `content_hash`：(kind, content_hash) 唯一索引承担去重——重复内容走
///   `INSERT .. ON CONFLICT DO UPDATE` 刷新原条目
const V1_CLIPBOARD_HISTORY: &str = "
CREATE TABLE clipboard_history (
    id           INTEGER PRIMARY KEY,
    kind         TEXT NOT NULL CHECK (kind IN ('text', 'image')),
    content_hash BLOB NOT NULL,
    text         TEXT,
    image_png    BLOB,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    recency      INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_clipboard_history_dedupe ON clipboard_history (kind, content_hash);
CREATE INDEX idx_clipboard_history_recency ON clipboard_history (recency);
";

/// 迁移 V2：密码本（ADR-0002）。
///
/// - `vault_meta`：单行（id 恒为 1）——Argon2id 盐值 + 主密码校验器（校验器
///   本身是 AES-GCM 密文，解密成功且明文匹配即主密码正确）
/// - `vault_entries`：凭据条目，整条载荷（标题/用户名/密码/URL/备注的 JSON）
///   经 AES-256-GCM 加密后存 `data`，库里不落任何明文
const V2_VAULT: &str = "
CREATE TABLE vault_meta (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    salt     BLOB NOT NULL,
    verifier BLOB NOT NULL
);
CREATE TABLE vault_entries (
    id         INTEGER PRIMARY KEY,
    data       BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
";

/// 顺序迁移：`MIGRATIONS[n]` 把 schema 从版本 n 升到 n+1。
const MIGRATIONS: &[&str] = &[V1_CLIPBOARD_HISTORY, V2_VAULT];

/// 打开（必要时创建）数据库文件并迁移到最新版本。
pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // 剪贴板历史与密码本各持一个连接共用此库：写冲突等锁而不是立刻报 BUSY
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    migrate(&conn)?;
    Ok(conn)
}

/// 打开内存数据库并迁移到最新版本（测试用）。
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

/// 把连接迁移到 `MIGRATIONS` 最新版本。幂等：已是最新则不做任何事。
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    migrate_with(conn, MIGRATIONS)
}

/// 逐条迁移，每条整体包在一个事务里：中途失败则回滚且 `user_version`
/// 停在原值，修复后重放从断点继续，不会留下半套 schema。
fn migrate_with(conn: &Connection, migrations: &[&str]) -> rusqlite::Result<()> {
    let version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (i, sql) in migrations.iter().enumerate().skip(version as usize) {
        // PRAGMA user_version 可在事务内设置并随事务回滚；i 来自枚举下标，无注入风险。
        if let Err(e) = conn.execute_batch(&format!(
            "BEGIN; {sql} PRAGMA user_version = {}; COMMIT;",
            i + 1
        )) {
            // execute_batch 失败不会自动回滚已执行的 BEGIN，需清理遗留的打开事务
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn fresh_database_migrates_to_latest() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();

        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());

        let table: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'clipboard_history'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table, "clipboard_history");

        let vault_meta: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'vault_meta'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(vault_meta, "vault_meta");

        let vault_entries: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'vault_entries'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(vault_entries, "vault_entries");
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        conn.execute(
            "INSERT INTO clipboard_history (kind, content_hash, text, created_at, updated_at, recency)
             VALUES ('text', x'0102', 'hello', 1, 1, 1)",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn open_reuses_existing_schema_without_error() {
        let dir = TempDir::new("db-test");
        let path = dir.path().join("test.db");

        {
            let conn = open(&path).unwrap();
            conn.execute(
                "INSERT INTO clipboard_history (kind, content_hash, text, created_at, updated_at, recency)
                 VALUES ('text', x'0304', 'persisted', 1, 1, 1)",
                [],
            )
            .unwrap();
        }
        {
            let conn = open(&path).unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn failed_migration_rolls_back_and_resumes_from_breakpoint() {
        let conn = Connection::open_in_memory().unwrap();
        // V1 成功、V2 非法：V1 应已提交（带版本号），V2 回滚
        let broken = &[V1_CLIPBOARD_HISTORY, "CREATE TABLE broken ( derp;"];
        assert!(migrate_with(&conn, broken).is_err());

        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        conn.execute(
            "INSERT INTO clipboard_history (kind, content_hash, text, created_at, updated_at, recency)
             VALUES ('text', x'0506', 'kept', 1, 1, 1)",
            [],
        )
        .unwrap();

        // 修复 V2 后重放：从断点继续，已有数据不受影响
        let fixed = &[V1_CLIPBOARD_HISTORY, "CREATE TABLE v2_table (id INTEGER);"];
        migrate_with(&conn, fixed).unwrap();
        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
