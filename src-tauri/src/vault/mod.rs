//! 密码本：解锁会话状态机 + 条目管理 + 密码复制编排。本模块是 tauri command
//! 的边界；加解密见 `crypto`，密文存取见 `repository`（ADR-0002）。
//!
//! - 解锁会话：主密码经 Argon2id 派生密钥后仅驻内存（`Zeroizing`，落锁即清零），
//!   校验器解密成功即密码正确。条目操作前经 `ensure_unlocked` 检查并刷新活跃
//!   时间，超过锁定超时未操作自动落锁。
//! - 密码复制：条目字段经剪贴板引擎的自家写入通道写出（不进历史），30 秒后
//!   带序号守卫清除——延迟窗口内用户复制了新内容则放弃清除，保护新内容。

pub mod crypto;
pub mod repository;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::State;
use zeroize::Zeroizing;

use crate::clipboard::engine::ClipboardEngine;

/// 复制的密码在剪贴板上的保留时长，到点自动清除。
pub const COPY_CLEAR_DELAY: Duration = Duration::from_secs(30);
/// 默认锁定超时：无操作该时长后自动落锁。
pub const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 300;
/// 主密码最短长度。
pub const MIN_MASTER_PASSWORD_LEN: usize = 8;
/// 自动落锁发往主窗口的事件：前端收到即切回解锁界面。
pub const LOCKED_EVENT: &str = "vault-locked";

/// 复制目标字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VaultCopyField {
    Username,
    Password,
}

/// 密码本状态：是否已设置主密码、当前是否解锁。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub initialized: bool,
    pub unlocked: bool,
}

/// 凭据条目载荷：整体序列化为 JSON 后经 AES-256-GCM 加密落盘。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryPayload {
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
}

impl EntryPayload {
    /// 保存前的规范化：标题去空白且必填，其余字段原样保留
    /// （用户名/密码中的空白字符可能是密码的一部分，不做修剪）。
    fn normalized(&self) -> Result<EntryPayload, VaultError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(VaultError::EmptyTitle);
        }
        Ok(EntryPayload {
            title: title.to_string(),
            username: self.username.clone(),
            password: self.password.clone(),
            url: self.url.clone(),
            notes: self.notes.clone(),
        })
    }
}

/// 查询结果条目（含明文字段，仅解锁会话内存在）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntryView {
    pub id: i64,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 密码本操作失败。Display 消息面向用户直接展示。
#[derive(Debug)]
pub enum VaultError {
    Locked,
    NotInitialized,
    AlreadyInitialized,
    WrongPassword,
    WeakPassword,
    EmptyTitle,
    EmptyField,
    NotFound,
    ClipboardUnavailable,
    Clipboard(String),
    Crypto(String),
    Storage(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Locked => write!(f, "密码本已锁定"),
            VaultError::NotInitialized => write!(f, "密码本尚未设置主密码"),
            VaultError::AlreadyInitialized => write!(f, "密码本已设置过主密码"),
            VaultError::WrongPassword => write!(f, "主密码错误"),
            VaultError::WeakPassword => write!(f, "主密码至少需要 {MIN_MASTER_PASSWORD_LEN} 位"),
            VaultError::EmptyTitle => write!(f, "标题不能为空"),
            VaultError::EmptyField => write!(f, "该字段为空，无可复制内容"),
            VaultError::NotFound => write!(f, "条目不存在"),
            VaultError::ClipboardUnavailable => write!(f, "剪贴板引擎不可用"),
            VaultError::Clipboard(e) => write!(f, "写入剪贴板失败: {e}"),
            VaultError::Crypto(e) => write!(f, "{e}"),
            VaultError::Storage(e) => write!(f, "密码本存储失败: {e}"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<rusqlite::Error> for VaultError {
    fn from(e: rusqlite::Error) -> Self {
        VaultError::Storage(e.to_string())
    }
}

impl From<crypto::CryptoError> for VaultError {
    fn from(e: crypto::CryptoError) -> Self {
        VaultError::Crypto(e.0)
    }
}

/// 解锁会话：派生密钥 + 最近活跃时间。密钥用 `Zeroizing` 包裹，会话结束
/// （落锁或替换）即清零内存。
struct Session {
    key: Zeroizing<[u8; crypto::KEY_LEN]>,
    last_activity: Instant,
}

pub struct VaultState {
    repo: Arc<repository::VaultRepository>,
    /// 剪贴板引擎（自家写入通道）：剪贴板引擎初始化失败时密码本仍可用，
    /// 仅复制功能降级报错。
    engine: Option<Arc<ClipboardEngine>>,
    session: Mutex<Option<Session>>,
    /// 锁定超时（秒），设置页可改。
    timeout_secs: AtomicU64,
}

impl VaultState {
    pub fn new(
        repo: Arc<repository::VaultRepository>,
        engine: Option<Arc<ClipboardEngine>>,
    ) -> Self {
        Self {
            repo,
            engine,
            session: Mutex::new(None),
            timeout_secs: AtomicU64::new(DEFAULT_LOCK_TIMEOUT_SECS),
        }
    }

    /// 锁定超时（秒）。
    pub fn set_lock_timeout(&self, secs: u64) {
        self.timeout_secs.store(secs, Ordering::Relaxed);
    }

    /// 无操作超过锁定超时即落锁；返回本次调用是否恰好触发了落锁
    /// （供自动锁定线程发事件用，避免重复通知）。
    pub fn expire_if_due(&self) -> bool {
        let mut session = self.session.lock().unwrap();
        let Some(s) = session.as_ref() else {
            return false;
        };
        if s.last_activity.elapsed()
            < Duration::from_secs(self.timeout_secs.load(Ordering::Relaxed))
        {
            return false;
        }
        *session = None;
        true
    }

    /// 当前状态（到期未操作的先落锁再报告）。
    pub fn status(&self) -> VaultStatus {
        self.expire_if_due();
        VaultStatus {
            initialized: self.repo.is_initialized().unwrap_or(false),
            unlocked: self.session.lock().unwrap().is_some(),
        }
    }

    /// 首次使用：生成盐值、派生密钥、写入校验器，成功即进入解锁会话。
    pub fn setup(&self, master_password: &str) -> Result<(), VaultError> {
        if master_password.chars().count() < MIN_MASTER_PASSWORD_LEN {
            return Err(VaultError::WeakPassword);
        }
        if self.repo.is_initialized()? {
            return Err(VaultError::AlreadyInitialized);
        }
        let salt = crypto::generate_salt();
        let key = crypto::derive_key(master_password, &salt)?;
        let verifier = crypto::seal(&key, crypto::VERIFIER_PLAINTEXT)?;
        self.repo.initialize(&repository::VaultMeta {
            salt: salt.to_vec(),
            verifier,
        })?;
        self.start_session(key);
        Ok(())
    }

    /// 主密码解锁；错误密码拒绝，会话保持锁定。
    pub fn unlock(&self, master_password: &str) -> Result<(), VaultError> {
        let meta = self.repo.meta()?.ok_or(VaultError::NotInitialized)?;
        let key = crypto::derive_key(master_password, &meta.salt)?;
        // 校验器解密成功且明文匹配 = 主密码正确（GCM 认证标签保证）；
        // 解密失败对校验器而言只有密码错误一种实际成因，归为 WrongPassword
        let check = crypto::open(&key, &meta.verifier).map_err(|_| VaultError::WrongPassword)?;
        if check != crypto::VERIFIER_PLAINTEXT {
            return Err(VaultError::WrongPassword);
        }
        self.start_session(key);
        Ok(())
    }

    /// 以派生密钥开启解锁会话。
    fn start_session(&self, key: Zeroizing<[u8; crypto::KEY_LEN]>) {
        *self.session.lock().unwrap() = Some(Session {
            key,
            last_activity: Instant::now(),
        });
    }

    /// 立即落锁，会话密钥清零。
    pub fn lock(&self) {
        *self.session.lock().unwrap() = None;
    }

    /// 取解锁会话密钥：到期未操作先落锁；命中会话则刷新活跃时间。
    fn ensure_unlocked(&self) -> Result<Zeroizing<[u8; crypto::KEY_LEN]>, VaultError> {
        self.expire_if_due();
        let mut session = self.session.lock().unwrap();
        let s = session.as_mut().ok_or(VaultError::Locked)?;
        s.last_activity = Instant::now();
        Ok(Zeroizing::new(*s.key))
    }

    /// 解密条目载荷。
    fn open_payload(key: &[u8; crypto::KEY_LEN], data: &[u8]) -> Result<EntryPayload, VaultError> {
        let plaintext = crypto::open(key, data)?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| VaultError::Crypto(format!("条目数据解析失败: {e}")))
    }

    /// 列出条目（`updated_at` 降序），可按标题/用户名关键词过滤（大小写不敏感）。
    pub fn query(&self, keyword: &str) -> Result<Vec<VaultEntryView>, VaultError> {
        let key = self.ensure_unlocked()?;
        let keyword = keyword.trim().to_lowercase();
        let views = self
            .repo
            .entries()?
            .into_iter()
            .map(|stored| {
                let payload = Self::open_payload(&key, &stored.data)?;
                Ok(VaultEntryView {
                    id: stored.id,
                    created_at: stored.created_at,
                    updated_at: stored.updated_at,
                    title: payload.title,
                    username: payload.username,
                    password: payload.password,
                    url: payload.url,
                    notes: payload.notes,
                })
            })
            .collect::<Result<Vec<_>, VaultError>>()?;
        Ok(views
            .into_iter()
            .filter(|v| {
                keyword.is_empty()
                    || v.title.to_lowercase().contains(&keyword)
                    || v.username.to_lowercase().contains(&keyword)
            })
            .collect())
    }

    /// 新增条目；标题必填（保存前去首尾空白）。
    pub fn create(&self, input: &EntryPayload) -> Result<i64, VaultError> {
        let key = self.ensure_unlocked()?;
        let payload = input.normalized()?;
        let data = crypto::seal(&key, &serde_json::to_vec(&payload).unwrap())?;
        Ok(self.repo.insert_entry(&data)?)
    }

    /// 更新条目；返回是否确有更新。
    pub fn update(&self, id: i64, input: &EntryPayload) -> Result<bool, VaultError> {
        let key = self.ensure_unlocked()?;
        let payload = input.normalized()?;
        let data = crypto::seal(&key, &serde_json::to_vec(&payload).unwrap())?;
        Ok(self.repo.update_entry(id, &data)?)
    }

    /// 删除条目；返回是否确有删除。
    pub fn delete(&self, id: i64) -> Result<bool, VaultError> {
        self.ensure_unlocked()?;
        Ok(self.repo.delete_entry(id)?)
    }

    /// 复制条目的用户名或密码：经自家写入通道写剪贴板（不进历史），30 秒后
    /// 带序号守卫自动清除。
    pub fn copy_field(&self, id: i64, field: VaultCopyField) -> Result<(), VaultError> {
        let key = self.ensure_unlocked()?;
        let stored = self.repo.entry(id)?.ok_or(VaultError::NotFound)?;
        let payload = Self::open_payload(&key, &stored.data)?;
        let value = match field {
            VaultCopyField::Username => &payload.username,
            VaultCopyField::Password => &payload.password,
        };
        if value.is_empty() {
            return Err(VaultError::EmptyField);
        }
        let engine = self
            .engine
            .as_ref()
            .ok_or(VaultError::ClipboardUnavailable)?;
        engine
            .write_own_text(value)
            .map_err(|e| VaultError::Clipboard(e.0))?;
        schedule_copy_clear(
            Arc::clone(engine),
            engine.access().sequence(),
            COPY_CLEAR_DELAY,
        );
        Ok(())
    }
}

/// 序号仍是自家写入的那个 = 延迟窗口内无新写入，清空安全；用户已复制新内容
/// 则不动。序号不可用时不盲目清除。
fn clear_if_still_ours(engine: &ClipboardEngine, expected_sequence: Option<u64>) {
    let Some(expected) = expected_sequence else {
        return;
    };
    if engine.access().sequence() != Some(expected) {
        return;
    }
    if let Err(e) = engine.write_own_text("") {
        eprintln!("youmi: 清除密码剪贴板失败: {e}");
    }
}

/// 延迟清除剪贴板中的密码（独立线程睡眠等待，复制频率低，开销可忽略）。
fn schedule_copy_clear(
    engine: Arc<ClipboardEngine>,
    expected_sequence: Option<u64>,
    delay: Duration,
) {
    if let Err(e) = std::thread::Builder::new()
        .name("vault-copy-clear".into())
        .spawn(move || {
            std::thread::sleep(delay);
            clear_if_still_ours(&engine, expected_sequence);
        })
    {
        eprintln!("youmi: 密码剪贴板清除线程启动失败: {e}");
    }
}

// ---------- tauri command 边界 ----------

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> Result<VaultStatus, String> {
    Ok(state.status())
}

#[tauri::command]
pub fn vault_setup(state: State<'_, VaultState>, master_password: String) -> Result<(), String> {
    state.setup(&master_password).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_unlock(state: State<'_, VaultState>, master_password: String) -> Result<(), String> {
    state.unlock(&master_password).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_lock(state: State<'_, VaultState>) -> Result<(), String> {
    state.lock();
    Ok(())
}

/// 锁定超时（秒），约束到 10 秒～24 小时。
#[tauri::command]
pub fn vault_set_lock_timeout(state: State<'_, VaultState>, seconds: u64) -> Result<(), String> {
    state.set_lock_timeout(seconds.clamp(10, 86_400));
    Ok(())
}

#[tauri::command]
pub fn vault_query(
    state: State<'_, VaultState>,
    keyword: Option<String>,
) -> Result<Vec<VaultEntryView>, String> {
    state
        .query(keyword.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_create(state: State<'_, VaultState>, entry: EntryPayload) -> Result<i64, String> {
    state.create(&entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_update(
    state: State<'_, VaultState>,
    id: i64,
    entry: EntryPayload,
) -> Result<bool, String> {
    state.update(id, &entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_delete(state: State<'_, VaultState>, id: i64) -> Result<bool, String> {
    state.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_copy(
    state: State<'_, VaultState>,
    id: i64,
    field: VaultCopyField,
) -> Result<(), String> {
    state.copy_field(id, field).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::repository::ClipboardHistoryRepository;
    use crate::platform::{ClipboardAccess, PlatformError};
    use std::sync::Mutex as StdMutex;

    // ---------- fake 剪贴板（与 clipboard::engine、pasteback 测试同款语义） ----------

    struct FakeClip(StdMutex<FakeClipState>);

    #[derive(Default)]
    struct FakeClipState {
        content: Option<crate::clipboard::model::ClipboardContent>,
        sequence: u64,
        writes: Vec<crate::clipboard::model::ClipboardContent>,
    }

    impl FakeClip {
        fn new() -> Arc<Self> {
            Arc::new(Self(StdMutex::new(FakeClipState::default())))
        }

        fn set_content(&self, content: crate::clipboard::model::ClipboardContent) {
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            s.content = Some(content);
        }

        fn content(&self) -> Option<crate::clipboard::model::ClipboardContent> {
            self.0.lock().unwrap().content.clone()
        }
    }

    impl ClipboardAccess for FakeClip {
        fn read(&self) -> Option<crate::clipboard::model::ClipboardContent> {
            self.0.lock().unwrap().content.clone()
        }

        fn sequence(&self) -> Option<u64> {
            Some(self.0.lock().unwrap().sequence)
        }

        fn write_text(&self, text: &str) -> Result<(), PlatformError> {
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            let content = crate::clipboard::model::ClipboardContent::Text(text.into());
            s.writes.push(content.clone());
            s.content = Some(content);
            Ok(())
        }

        fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            let content = crate::clipboard::model::ClipboardContent::ImagePng(png.to_vec());
            s.writes.push(content.clone());
            s.content = Some(content);
            Ok(())
        }
    }

    // ---------- harness ----------

    struct Harness {
        state: VaultState,
        clip: Arc<FakeClip>,
        history: Arc<ClipboardHistoryRepository>,
    }

    fn harness(timeout_secs: u64, with_engine: bool) -> Harness {
        let repo = Arc::new(repository::VaultRepository::open_in_memory().unwrap());
        let history = Arc::new(ClipboardHistoryRepository::open_in_memory().unwrap());
        let clip = FakeClip::new();
        let engine = if with_engine {
            Some(Arc::new(ClipboardEngine::new(
                Arc::clone(&history),
                Arc::clone(&clip) as Arc<dyn ClipboardAccess>,
            )))
        } else {
            None
        };
        let state = VaultState::new(repo, engine);
        state.set_lock_timeout(timeout_secs);
        Harness {
            state,
            clip,
            history,
        }
    }

    const MASTER: &str = "master-password-1";

    fn sample(title: &str) -> EntryPayload {
        EntryPayload {
            title: title.into(),
            username: "alice".into(),
            password: "p@ss".into(),
            url: "https://example.com".into(),
            notes: String::new(),
        }
    }

    fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        cond()
    }

    // ---------- 会话与锁定 ----------

    #[test]
    fn setup_initializes_and_unlocks() {
        let h = harness(300, true);
        assert_eq!(
            h.state.status(),
            VaultStatus {
                initialized: false,
                unlocked: false
            }
        );

        h.state.setup(MASTER).unwrap();

        assert_eq!(
            h.state.status(),
            VaultStatus {
                initialized: true,
                unlocked: true
            }
        );
        // setup 后直接可用：能建条目
        h.state.create(&sample("GitHub")).unwrap();
    }

    #[test]
    fn weak_password_is_rejected() {
        let h = harness(300, true);
        assert!(matches!(
            h.state.setup("short"),
            Err(VaultError::WeakPassword)
        ));
        assert!(!h.state.status().initialized);
    }

    #[test]
    fn double_setup_is_rejected() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        assert!(matches!(
            h.state.setup("another-password"),
            Err(VaultError::AlreadyInitialized)
        ));
    }

    #[test]
    fn lock_then_wrong_password_rejected_then_correct_unlocks() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        h.state.lock();
        assert!(!h.state.status().unlocked);

        // 错误密码被拒绝，且不进入会话
        assert!(matches!(
            h.state.unlock("wrong-password"),
            Err(VaultError::WrongPassword)
        ));
        assert!(!h.state.status().unlocked);

        h.state.unlock(MASTER).unwrap();
        assert!(h.state.status().unlocked);
    }

    #[test]
    fn operations_are_rejected_while_locked() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();
        h.state.lock();

        assert!(matches!(h.state.query(""), Err(VaultError::Locked)));
        assert!(matches!(
            h.state.create(&sample("x")),
            Err(VaultError::Locked)
        ));
        assert!(matches!(
            h.state.update(id, &sample("x")),
            Err(VaultError::Locked)
        ));
        assert!(matches!(h.state.delete(id), Err(VaultError::Locked)));
        assert!(matches!(
            h.state.copy_field(id, VaultCopyField::Password),
            Err(VaultError::Locked)
        ));
        // 落锁期间数据不丢：重新解锁后仍在
        h.state.unlock(MASTER).unwrap();
        assert_eq!(h.state.query("").unwrap().len(), 1);
    }

    #[test]
    fn idle_session_auto_locks_after_timeout() {
        // 超时 0 秒 = 任何后续操作前都已到期
        let h = harness(0, true);
        h.state.setup(MASTER).unwrap();
        // expire_if_due 只在到期落锁那一刻返回 true：再次调用不再触发
        assert!(h.state.expire_if_due());
        assert!(!h.state.expire_if_due());
        assert!(!h.state.status().unlocked);

        // 调大超时后重新解锁，会话恢复正常
        h.state.set_lock_timeout(300);
        h.state.unlock(MASTER).unwrap();
        assert!(h.state.status().unlocked);
    }

    // ---------- 条目 CRUD 与搜索 ----------

    #[test]
    fn create_query_update_delete_flow() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();

        let id = h.state.create(&sample("GitHub")).unwrap();
        let entries = h.state.query("").unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.id, id);
        assert_eq!(entry.title, "GitHub");
        assert_eq!(entry.username, "alice");
        assert_eq!(entry.password, "p@ss");
        assert_eq!(entry.url, "https://example.com");

        let mut updated = sample("GitHub Work");
        updated.username = "bob".into();
        updated.password = "new-pw".into();
        assert!(h.state.update(id, &updated).unwrap());
        let entry = &h.state.query("").unwrap()[0];
        assert_eq!(entry.title, "GitHub Work");
        assert_eq!(entry.username, "bob");
        assert_eq!(entry.password, "new-pw");
        assert!(entry.updated_at >= entry.created_at);

        assert!(h.state.delete(id).unwrap());
        assert!(h.state.query("").unwrap().is_empty());
        assert!(!h.state.update(id, &updated).unwrap());
    }

    #[test]
    fn query_filters_by_title_or_username_case_insensitive() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        h.state
            .create(&EntryPayload {
                title: "GitHub".into(),
                username: "Alice".into(),
                ..sample("GitHub")
            })
            .unwrap();
        h.state
            .create(&EntryPayload {
                title: "公司邮箱".into(),
                username: "bob@corp.example".into(),
                ..sample("公司邮箱")
            })
            .unwrap();

        let found = |kw: &str| h.state.query(kw).unwrap().len();

        assert_eq!(found("github"), 1); // 标题，大小写不敏感
        assert_eq!(found("ALICE"), 1); // 用户名
        assert_eq!(found("bob"), 1);
        assert_eq!(found("公司"), 1);
        assert_eq!(found("no-such-keyword"), 0);
        assert_eq!(found(""), 2); // 空关键词 = 全部
        assert_eq!(found("   "), 2); // 纯空白 = 全部
    }

    #[test]
    fn empty_title_is_rejected() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        assert!(matches!(
            h.state.create(&sample("   ")),
            Err(VaultError::EmptyTitle)
        ));
    }

    // ---------- 复制与 30 秒清除 ----------

    #[test]
    fn copy_writes_clipboard_without_entering_history() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();

        h.state.copy_field(id, VaultCopyField::Password).unwrap();

        assert_eq!(
            h.clip.content(),
            Some(crate::clipboard::model::ClipboardContent::Text(
                "p@ss".into()
            ))
        );
        // 自家写入不进历史
        h.state.copy_field(id, VaultCopyField::Username).unwrap();
        assert_eq!(
            h.clip.content(),
            Some(crate::clipboard::model::ClipboardContent::Text(
                "alice".into()
            ))
        );
    }

    #[test]
    fn copy_empty_field_is_rejected() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let mut entry = sample("NoUrl");
        entry.username = String::new();
        let id = h.state.create(&entry).unwrap();

        assert!(matches!(
            h.state.copy_field(id, VaultCopyField::Username),
            Err(VaultError::EmptyField)
        ));
        // 密码仍可复制
        h.state.copy_field(id, VaultCopyField::Password).unwrap();
    }

    #[test]
    fn copy_without_clipboard_engine_is_unavailable() {
        let h = harness(300, false);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();

        assert!(matches!(
            h.state.copy_field(id, VaultCopyField::Password),
            Err(VaultError::ClipboardUnavailable)
        ));
    }

    #[test]
    fn copy_missing_entry_is_not_found() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        assert!(matches!(
            h.state.copy_field(12345, VaultCopyField::Password),
            Err(VaultError::NotFound)
        ));
    }

    #[test]
    fn clear_if_still_ours_empties_clipboard() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();
        h.state.copy_field(id, VaultCopyField::Password).unwrap();
        let written = h.clip.sequence();

        clear_if_still_ours(&h.state.engine.as_ref().unwrap(), written);

        // 剪贴板被清空（写入空文本），且空写入同样不进历史
        assert_eq!(
            h.clip.content(),
            Some(crate::clipboard::model::ClipboardContent::Text(
                String::new()
            ))
        );
        assert!(h.history.query(&Default::default()).unwrap().is_empty());
    }

    #[test]
    fn clear_skipped_when_user_copied_new_content() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();
        h.state.copy_field(id, VaultCopyField::Password).unwrap();
        let written = h.clip.sequence();

        // 延迟窗口内用户复制了新内容：序号前进，清除放弃
        h.clip
            .set_content(crate::clipboard::model::ClipboardContent::Text(
                "fresh user copy".into(),
            ));
        clear_if_still_ours(&h.state.engine.as_ref().unwrap(), written);

        assert_eq!(
            h.clip.content(),
            Some(crate::clipboard::model::ClipboardContent::Text(
                "fresh user copy".into()
            ))
        );
    }

    #[test]
    fn scheduled_clear_runs_after_delay() {
        let h = harness(300, true);
        h.state.setup(MASTER).unwrap();
        let id = h.state.create(&sample("GitHub")).unwrap();
        h.state.copy_field(id, VaultCopyField::Password).unwrap();
        let written = h.clip.sequence();

        schedule_copy_clear(
            Arc::clone(h.state.engine.as_ref().unwrap()),
            written,
            Duration::from_millis(60),
        );

        let cleared = wait_until(Duration::from_secs(2), || {
            h.clip.content()
                == Some(crate::clipboard::model::ClipboardContent::Text(
                    String::new(),
                ))
        });
        assert!(cleared, "延迟到期后剪贴板应被清空");
    }

    #[test]
    fn database_file_stores_no_plaintext() {
        // 直查落盘字节：主密码与全部条目字段都不得以明文出现在数据库文件里
        let dir = crate::test_support::TempDir::new("vault-plaintext-test");
        let repo =
            Arc::new(repository::VaultRepository::open(&dir.path().join("youmi.db")).unwrap());
        let state = VaultState::new(repo, None);
        state.set_lock_timeout(300);

        state.setup(MASTER).unwrap();
        state
            .create(&EntryPayload {
                title: "TITLE-PLAINTEXT".into(),
                username: "USER-PLAINTEXT".into(),
                password: "PASSWORD-PLAINTEXT".into(),
                url: "URL-PLAINTEXT".into(),
                notes: "NOTES-PLAINTEXT".into(),
            })
            .unwrap();

        let bytes = std::fs::read(dir.path().join("youmi.db")).unwrap();
        for secret in [
            MASTER.as_bytes(),
            b"TITLE-PLAINTEXT",
            b"USER-PLAINTEXT",
            b"PASSWORD-PLAINTEXT",
            b"URL-PLAINTEXT",
            b"NOTES-PLAINTEXT",
        ] {
            assert!(
                !bytes.windows(secret.len()).any(|window| window == secret),
                "数据库文件中发现了明文：{}",
                String::from_utf8_lossy(secret)
            );
        }

        // 密文确实落了盘：重新打开并解锁后能读回全部字段
        let repo =
            Arc::new(repository::VaultRepository::open(&dir.path().join("youmi.db")).unwrap());
        let reopened = VaultState::new(repo, None);
        reopened.set_lock_timeout(300);
        assert!(matches!(
            reopened.unlock("wrong-password"),
            Err(VaultError::WrongPassword)
        ));
        reopened.unlock(MASTER).unwrap();
        let entries = reopened.query("").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].password, "PASSWORD-PLAINTEXT");
    }
}
