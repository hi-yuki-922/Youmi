//! SQLite 数据库：连接打开 + migration 机制。
//!
//! 剪贴板历史与密码本（ticket 08）密文共用一库；新增 schema 一律在本文件
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

/// 顺序迁移：`MIGRATIONS[n]` 把 schema 从版本 n 升到 n+1。
const MIGRATIONS: &[&str] = &[V1_CLIPBOARD_HISTORY];

/// 打开（必要时创建）数据库文件并迁移到最新版本。
pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
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
    let version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(sql)?;
        // PRAGMA 不支持绑定参数；i 来自枚举下标，无注入风险。
        conn.execute_batch(&format!("PRAGMA user_version = {};", i + 1))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = std::env::temp_dir().join(format!(
            "youmi-db-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");

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

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
