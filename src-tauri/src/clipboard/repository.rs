//! 剪贴板历史 Repository：SQLite 之上的深模块。
//!
//! 外部行为（也是测试断言的全部）：
//! - `capture`：内容入库；重复内容（同类型同指纹）刷新原条目到顶（created_at 不变）
//! - 容量：默认 1000、可配置，超出淘汰 recency 最旧的条目
//! - `query`：关键词（仅文本条目）、类型过滤、recency 降序；不含图片字节
//! - `image_data` / `delete` / `clear` / `len`

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::model::{ClipboardContent, ClipboardKind};
use crate::db;

/// 默认容量：超出后按 recency 淘汰最旧条目。
pub const DEFAULT_CAPACITY: usize = 1000;

/// 查询结果条目：不含图片字节（大对象经 `image_data` 按需取）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: i64,
    pub kind: ClipboardKind,
    pub text: Option<String>,
    pub has_image: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 历史查询条件。
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    /// 关键词：仅匹配文本条目（SQLite LIKE，ASCII 大小写不敏感）。
    pub keyword: Option<String>,
    pub kind: Option<ClipboardKind>,
    pub limit: u32,
}

pub struct ClipboardHistoryRepository {
    conn: Mutex<Connection>,
    capacity: AtomicUsize,
}

impl ClipboardHistoryRepository {
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
            capacity: AtomicUsize::new(DEFAULT_CAPACITY),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    /// 调整容量并立即按新容量淘汰（缩容即刻生效，不等下次捕获）。
    /// 容量 0 语义为"不再保留任何条目"（下次捕获即清空）；调用方（command 层）
    /// 应约束到合理区间。
    pub fn set_capacity(&self, capacity: usize) -> rusqlite::Result<()> {
        self.capacity.store(capacity, Ordering::Relaxed);
        let conn = self.conn.lock().unwrap();
        evict(&conn, capacity).map(|_| ())
    }

    /// 入库或刷新重复条目到顶，然后按容量淘汰最旧。返回条目 id。
    pub fn capture(&self, content: &ClipboardContent) -> rusqlite::Result<i64> {
        let hash = content.fingerprint();
        let now = now_ms();
        let kind = content.kind().as_str();
        let (text, image_png) = match content {
            ClipboardContent::Text(text) => (Some(text.as_str()), None::<&[u8]>),
            ClipboardContent::ImagePng(bytes) => (None, Some(bytes.as_slice())),
        };

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let id: i64 = tx.query_row(
            "INSERT INTO clipboard_history (kind, content_hash, text, image_png, created_at, updated_at, recency)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, (SELECT IFNULL(MAX(recency), 0) + 1 FROM clipboard_history))
             ON CONFLICT(kind, content_hash) DO UPDATE SET
                 updated_at = excluded.updated_at,
                 recency = (SELECT IFNULL(MAX(recency), 0) + 1 FROM clipboard_history)
             RETURNING id",
            params![kind, hash, text, image_png, now],
            |row| row.get(0),
        )?;
        evict(&tx, self.capacity.load(Ordering::Relaxed))?;
        tx.commit()?;
        Ok(id)
    }

    /// 按条件查询历史，recency 降序（最新在前）。不含图片字节。
    pub fn query(&self, query: &HistoryQuery) -> rusqlite::Result<Vec<ClipboardEntry>> {
        // 关键词做 LIKE 转义（\ % _），只匹配文本条目：图片条目 text 为 NULL，自动被 LIKE 过滤。
        let pattern = query.keyword.as_ref().map(|kw| {
            let mut escaped = String::with_capacity(kw.len() + 2);
            escaped.push('%');
            for ch in kw.chars() {
                if matches!(ch, '\\' | '%' | '_') {
                    escaped.push('\\');
                }
                escaped.push(ch);
            }
            escaped.push('%');
            escaped
        });
        // 0 = 不限制；UI 默认值由 command 层持有，repository 不藏第二套默认
        let limit: i64 = if query.limit == 0 {
            -1
        } else {
            i64::from(query.limit)
        };

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, kind, text, (image_png IS NOT NULL), created_at, updated_at
             FROM clipboard_history
             WHERE (?1 IS NULL OR text LIKE ?1 ESCAPE '\\')
               AND (?2 IS NULL OR kind = ?2)
             ORDER BY recency DESC
             LIMIT ?3",
        )?;
        let kind_str = query.kind.map(|k| k.as_str());
        let rows = stmt.query_map(params![pattern, kind_str, limit], map_entry)?;
        rows.collect()
    }

    /// 取条目的 PNG 字节；文本条目或不存在时返回 None。
    pub fn image_data(&self, id: i64) -> rusqlite::Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT image_png FROM clipboard_history WHERE id = ?1 AND kind = 'image'",
            params![id],
            |row| row.get(0),
        )
        .optional()
    }

    /// 取条目的完整内容（回帖用）：文本或图片 PNG 字节；不存在返回 None。
    pub fn entry_content(&self, id: i64) -> rusqlite::Result<Option<ClipboardContent>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT kind, text, image_png FROM clipboard_history WHERE id = ?1",
            params![id],
            |row| {
                let kind: ClipboardKind =
                    row.get::<_, String>(0)?.parse().map_err(|e: String| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            e.into(),
                        )
                    })?;
                Ok(match kind {
                    ClipboardKind::Image => ClipboardContent::ImagePng(row.get(2)?),
                    ClipboardKind::Text => {
                        ClipboardContent::Text(row.get::<_, Option<String>>(1)?.unwrap_or_default())
                    }
                })
            },
        )
        .optional()
    }

    /// 删除单条；返回是否确有删除。
    pub fn delete(&self, id: i64) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    /// 清空历史；返回删除的条数。
    pub fn clear(&self) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_history", [])
    }

    /// 当前条目总数。
    pub fn len(&self) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|n| n as usize)
    }

    pub fn is_empty(&self) -> rusqlite::Result<bool> {
        self.len().map(|n| n == 0)
    }
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let kind: ClipboardKind = row.get::<_, String>(1)?.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, e.into())
    })?;
    Ok(ClipboardEntry {
        id: row.get(0)?,
        kind,
        text: row.get(2)?,
        has_image: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

/// 淘汰 recency 不在 top-`capacity` 的条目（capture 事务内与 set_capacity 共用）。
fn evict(conn: &Connection, capacity: usize) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM clipboard_history WHERE recency NOT IN (
             SELECT recency FROM clipboard_history ORDER BY recency DESC LIMIT ?1
         )",
        params![capacity as i64],
    )
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn repo() -> ClipboardHistoryRepository {
        ClipboardHistoryRepository::open_in_memory().unwrap()
    }

    fn text(s: &str) -> ClipboardContent {
        ClipboardContent::Text(s.into())
    }

    fn png(bytes: &[u8]) -> ClipboardContent {
        ClipboardContent::ImagePng(bytes.to_vec())
    }

    /// 依次捕获多个内容，返回各自 id。
    fn capture_all(repo: &ClipboardHistoryRepository, contents: &[ClipboardContent]) {
        for content in contents {
            repo.capture(content).unwrap();
        }
    }

    /// 断言当前历史的文本序列（最新在前）；图片条目用 "(image)" 占位。
    fn assert_order(repo: &ClipboardHistoryRepository, expected: &[&str]) {
        let entries = repo.query(&query(None, None, u32::MAX)).unwrap();
        let actual: Vec<String> = entries
            .into_iter()
            .map(|e| e.text.unwrap_or_else(|| "(image)".into()))
            .collect();
        assert_eq!(actual, expected);
    }

    fn query(keyword: Option<&str>, kind: Option<ClipboardKind>, limit: u32) -> HistoryQuery {
        HistoryQuery {
            keyword: keyword.map(str::to_string),
            kind,
            limit,
        }
    }

    #[test]
    fn captured_text_is_queryable_newest_first() {
        let repo = repo();
        capture_all(&repo, &[text("first"), text("second")]);
        assert_order(&repo, &["second", "first"]);
    }

    #[test]
    fn captured_image_stores_png_blob() {
        let repo = repo();
        let blob = vec![0x89u8, b'P', b'N', b'G', 1, 2, 3];
        capture_all(&repo, &[text("a note"), png(&blob)]);

        let entries = repo.query(&query(None, None, u32::MAX)).unwrap();
        assert_eq!(entries.len(), 2);
        let image_entry = entries
            .iter()
            .find(|e| e.kind == ClipboardKind::Image)
            .unwrap();
        assert_eq!(image_entry.text, None);
        assert!(image_entry.has_image);
        assert_eq!(
            repo.image_data(image_entry.id).unwrap().as_deref(),
            Some(blob.as_slice())
        );

        let text_entry = entries
            .iter()
            .find(|e| e.kind == ClipboardKind::Text)
            .unwrap();
        assert!(!text_entry.has_image);
        assert_eq!(repo.image_data(text_entry.id).unwrap(), None);
    }

    #[test]
    fn duplicate_capture_refreshes_single_entry_to_top() {
        let repo = repo();
        repo.capture(&text("dup")).unwrap();
        let original = &repo.query(&query(None, None, u32::MAX)).unwrap()[0];
        let original_id = original.id;
        let original_created_at = original.created_at;
        let original_updated_at = original.updated_at;

        capture_all(&repo, &[text("other"), text("dup")]);

        assert_eq!(repo.len().unwrap(), 2);
        assert_order(&repo, &["dup", "other"]);

        // 刷新保留原条目（同一行、原 created_at），仅更新 updated_at。
        let entries = repo.query(&query(None, None, u32::MAX)).unwrap();
        let dup = &entries[0];
        assert_eq!(dup.text.as_deref(), Some("dup"));
        assert_eq!(dup.id, original_id);
        assert_eq!(dup.created_at, original_created_at);
        assert!(dup.updated_at > original_updated_at);
    }

    #[test]
    fn duplicate_image_capture_stays_single_entry() {
        let repo = repo();
        let blob = vec![9u8, 8, 7];
        capture_all(&repo, &[png(&blob), text("x"), png(&blob)]);
        assert_eq!(repo.len().unwrap(), 2);
        assert_order(&repo, &["(image)", "x"]);
    }

    #[test]
    fn capacity_evicts_oldest_beyond_limit() {
        let repo = repo();
        repo.set_capacity(3).unwrap();
        capture_all(&repo, &[text("a"), text("b"), text("c"), text("d")]);

        assert_eq!(repo.len().unwrap(), 3);
        assert_order(&repo, &["d", "c", "b"]); // 最旧的 a 被淘汰
    }

    #[test]
    fn refreshed_old_entry_survives_eviction() {
        let repo = repo();
        repo.set_capacity(3).unwrap();
        capture_all(&repo, &[text("a"), text("b"), text("c")]);
        repo.capture(&text("a")).unwrap(); // a 刷新到顶
        capture_all(&repo, &[text("d")]);

        assert_order(&repo, &["d", "a", "c"]); // b 成为最旧被淘汰
    }

    #[test]
    fn capacity_is_configurable_at_runtime() {
        let repo = repo();
        assert_eq!(repo.capacity(), DEFAULT_CAPACITY);
        repo.set_capacity(1).unwrap();
        capture_all(&repo, &[text("a"), text("b")]);
        assert_eq!(repo.len().unwrap(), 1);
        assert_order(&repo, &["b"]);
    }

    #[test]
    fn shrinking_capacity_evicts_immediately() {
        let repo = repo();
        capture_all(&repo, &[text("a"), text("b"), text("c")]);
        repo.set_capacity(1).unwrap();
        assert_eq!(repo.len().unwrap(), 1);
        assert_order(&repo, &["c"]); // 只留最新
    }

    #[test]
    fn keyword_search_matches_text_entries_only() {
        let repo = repo();
        capture_all(
            &repo,
            &[text("hello world"), text("rust 剪贴板"), png(&[1u8])],
        );

        let found = |kw: &str| -> Vec<String> {
            repo.query(&query(Some(kw), None, u32::MAX))
                .unwrap()
                .into_iter()
                .map(|e| e.text.unwrap())
                .collect()
        };

        assert_eq!(found("hello"), vec!["hello world".to_string()]);
        assert_eq!(found("剪贴"), vec!["rust 剪贴板".to_string()]);
        assert!(found("no-such-keyword").is_empty());
        // LIKE 通配符按字面匹配
        assert_eq!(found("hello world"), vec!["hello world".to_string()]);
    }

    #[test]
    fn kind_filter_narrows_results() {
        let repo = repo();
        capture_all(&repo, &[text("t1"), png(&[1u8]), text("t2")]);

        let texts = repo
            .query(&query(None, Some(ClipboardKind::Text), u32::MAX))
            .unwrap();
        assert_eq!(
            texts.iter().map(|e| e.kind).collect::<Vec<_>>(),
            vec![ClipboardKind::Text, ClipboardKind::Text]
        );

        let images = repo
            .query(&query(None, Some(ClipboardKind::Image), u32::MAX))
            .unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].kind, ClipboardKind::Image);
    }

    #[test]
    fn entry_content_returns_full_content() {
        let repo = repo();
        let blob = vec![5u8, 6, 7];
        capture_all(&repo, &[text("hello"), png(&blob)]);

        let entries = repo.query(&query(None, None, u32::MAX)).unwrap();
        let text_entry = entries
            .iter()
            .find(|e| e.kind == ClipboardKind::Text)
            .unwrap();
        let image_entry = entries
            .iter()
            .find(|e| e.kind == ClipboardKind::Image)
            .unwrap();

        assert_eq!(
            repo.entry_content(text_entry.id).unwrap(),
            Some(text("hello"))
        );
        assert_eq!(
            repo.entry_content(image_entry.id).unwrap(),
            Some(png(&blob))
        );
        assert_eq!(repo.entry_content(-1).unwrap(), None);
    }

    #[test]
    fn delete_single_entry() {
        let repo = repo();
        capture_all(&repo, &[text("keep"), text("gone")]);
        let entries = repo.query(&query(None, None, u32::MAX)).unwrap();
        let gone_id = entries
            .iter()
            .find(|e| e.text.as_deref() == Some("gone"))
            .unwrap()
            .id;

        assert!(repo.delete(gone_id).unwrap());
        assert_eq!(repo.len().unwrap(), 1);
        assert_order(&repo, &["keep"]);
        assert!(!repo.delete(gone_id).unwrap()); // 二次删除返回 false
    }

    #[test]
    fn clear_removes_everything() {
        let repo = repo();
        capture_all(&repo, &[text("a"), png(&[1u8]), text("b")]);
        assert_eq!(repo.clear().unwrap(), 3);
        assert!(repo.is_empty().unwrap());
    }

    #[test]
    fn history_persists_across_reopen() {
        let dir = TempDir::new("repo-test");
        let path = dir.path().join("clipboard.db");

        {
            let repo = ClipboardHistoryRepository::open(&path).unwrap();
            repo.set_capacity(2).unwrap();
            capture_all(&repo, &[text("first"), text("second"), text("third")]);
        }
        {
            let repo = ClipboardHistoryRepository::open(&path).unwrap();
            assert_eq!(repo.len().unwrap(), 2); // 容量 2 下的最终状态
            assert_order(&repo, &["third", "second"]);
        }
    }
}
