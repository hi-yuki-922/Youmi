//! 捕获引擎：把剪贴板变化接到 Repository，并实现"自家写入通道"。
//!
//! - `handle_change`：监听线程回调 → 读取剪贴板 → 过滤自家写入 → 入库
//! - `write_own_*`：Youmi 自身写剪贴板的唯一入口（密码复制、划词模拟复制等），
//!   经此通道写入的内容不进入历史。
//!
//! 自家写入识别用剪贴板序号（Windows 的 GetClipboardSequenceNumber，每次内容
//! 变化递增且永不复用）：登记写入完成时的序号，变化事件按当前序号过滤。一次
//! 自家写入（empty + SetClipboardData）会触发多个事件但序号相同，全部命中过滤，
//! 因此标记无需消费；用户稍后复制相同内容时序号已前进，正常入库。
//! 写入调用本身到序号登记之间由 pending 标志兜底，封住事件乱序的竞态窗口。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::platform::{ClipboardAccess, PlatformError};

use super::model::ClipboardContent;
use super::repository::ClipboardHistoryRepository;

/// pending 标志的保鲜期：write_own 异常终止时的兜底，避免永久误伤。
const PENDING_TTL: Duration = Duration::from_secs(5);

/// 保留的自家写入序号上限（自家写入本身低频，仅需防极端膨胀）。
const MAX_SEQUENCES: usize = 64;

pub struct ClipboardEngine {
    repo: Arc<ClipboardHistoryRepository>,
    access: Arc<dyn ClipboardAccess>,
    self_writes: Mutex<SelfWriteRegistry>,
}

impl ClipboardEngine {
    pub fn new(repo: Arc<ClipboardHistoryRepository>, access: Arc<dyn ClipboardAccess>) -> Self {
        Self {
            repo,
            access,
            self_writes: Mutex::new(SelfWriteRegistry::new()),
        }
    }

    /// 剪贴板变化回调（监听线程上执行）。
    pub fn handle_change(&self) {
        let Some(content) = self.access.read() else {
            return; // 非文本/图片内容或读取瞬间失败：不捕获
        };
        if let ClipboardContent::Text(text) = &content {
            if text.trim().is_empty() {
                return; // 空文本/纯空白不入库
            }
        }
        if let Some(sequence) = self.access.sequence() {
            if self.self_writes.lock().unwrap().matches(sequence) {
                return; // 自家写入引发的变化，不入库
            }
        }
        if let Err(e) = self.repo.capture(&content) {
            eprintln!("youmi: 剪贴板入库失败: {e}");
        }
    }

    /// 自家写入通道：写文本，且不进入剪贴板历史。
    pub fn write_own_text(&self, text: &str) -> Result<(), PlatformError> {
        self.write_own(&ClipboardContent::Text(text.to_string()))
    }

    /// 自家写入通道：写 PNG 图片，且不进入剪贴板历史。
    pub fn write_own_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
        self.write_own(&ClipboardContent::ImagePng(png.to_vec()))
    }

    /// 自家写入通道的唯一入口：写内容且不进入剪贴板历史（回帖写条目、
    /// 恢复用户原剪贴板都走这里）。
    pub fn write_own(&self, content: &ClipboardContent) -> Result<(), PlatformError> {
        self.self_writes.lock().unwrap().begin();
        let result = match content {
            ClipboardContent::Text(text) => self.access.write_text(text),
            ClipboardContent::ImagePng(bytes) => self.access.write_image_png(bytes),
        };
        let mut registry = self.self_writes.lock().unwrap();
        match (&result, self.access.sequence()) {
            (Ok(()), Some(sequence)) => registry.commit(sequence),
            _ => registry.rollback(), // 写入失败，或平台拿不到序号时放弃过滤
        }
        result
    }

    /// 底层剪贴板访问：回帖读取用户原剪贴板与序号用（与捕获同源，自家写入
    /// 过滤才能对上）。
    pub fn access(&self) -> &Arc<dyn ClipboardAccess> {
        &self.access
    }
}

/// 自家写入识别表：写入进行中的 pending 标志 + 已完成写入的剪贴板序号。
struct SelfWriteRegistry {
    pending_since: Option<Instant>,
    sequences: VecDeque<u64>,
}

impl SelfWriteRegistry {
    fn new() -> Self {
        Self {
            pending_since: None,
            sequences: VecDeque::new(),
        }
    }

    fn begin(&mut self) {
        self.pending_since = Some(Instant::now());
    }

    fn commit(&mut self, sequence: u64) {
        self.pending_since = None;
        self.sequences.push_back(sequence);
        while self.sequences.len() > MAX_SEQUENCES {
            self.sequences.pop_front();
        }
    }

    fn rollback(&mut self) {
        self.pending_since = None;
    }

    fn matches(&mut self, sequence: u64) -> bool {
        if let Some(since) = self.pending_since {
            if since.elapsed() < PENDING_TTL {
                return true;
            }
            self.pending_since = None; // 写入方异常终止的兜底
        }
        self.sequences.contains(&sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex as StdMutex;

    /// 假剪贴板：记录写入、按需吐内容、可注入失败；序号随每次内容变化递增，
    /// 与真实剪贴板语义一致。
    struct FakeAccess {
        state: StdMutex<FakeState>,
        fail_writes: AtomicBool,
        sequence: AtomicU64,
    }

    #[derive(Default)]
    struct FakeState {
        content: Option<ClipboardContent>,
        writes: Vec<ClipboardContent>,
    }

    impl FakeAccess {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                state: StdMutex::new(FakeState::default()),
                fail_writes: AtomicBool::new(false),
                sequence: AtomicU64::new(100),
            })
        }

        fn set_content(&self, content: ClipboardContent) {
            // 模拟一次外部写入：内容与序号一起变化
            self.sequence.fetch_add(1, Ordering::Relaxed);
            self.state.lock().unwrap().content = Some(content);
        }

        fn writes(&self) -> Vec<ClipboardContent> {
            self.state.lock().unwrap().writes.clone()
        }

        fn set_fail_writes(&self, fail: bool) {
            self.fail_writes.store(fail, Ordering::Relaxed);
        }
    }

    impl ClipboardAccess for FakeAccess {
        fn read(&self) -> Option<ClipboardContent> {
            self.state.lock().unwrap().content.clone()
        }

        fn sequence(&self) -> Option<u64> {
            Some(self.sequence.load(Ordering::Relaxed))
        }

        fn write_text(&self, text: &str) -> Result<(), PlatformError> {
            if self.fail_writes.load(Ordering::Relaxed) {
                return Err(PlatformError("写入失败（测试注入）".into()));
            }
            self.sequence.fetch_add(1, Ordering::Relaxed);
            let content = ClipboardContent::Text(text.to_string());
            let mut state = self.state.lock().unwrap();
            state.writes.push(content.clone());
            state.content = Some(content);
            Ok(())
        }

        fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
            if self.fail_writes.load(Ordering::Relaxed) {
                return Err(PlatformError("写入失败（测试注入）".into()));
            }
            self.sequence.fetch_add(1, Ordering::Relaxed);
            let content = ClipboardContent::ImagePng(png.to_vec());
            let mut state = self.state.lock().unwrap();
            state.writes.push(content.clone());
            state.content = Some(content);
            Ok(())
        }
    }

    fn engine(access: Arc<FakeAccess>) -> ClipboardEngine {
        let repo = Arc::new(ClipboardHistoryRepository::open_in_memory().unwrap());
        ClipboardEngine::new(repo, access)
    }

    fn texts(engine: &ClipboardEngine) -> Vec<String> {
        engine
            .repo
            .query(&super::super::repository::HistoryQuery::default())
            .unwrap()
            .into_iter()
            .map(|e| e.text.unwrap_or_else(|| "(image)".into()))
            .collect()
    }

    #[test]
    fn clipboard_change_gets_captured() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());
        access.set_content(ClipboardContent::Text("copied".into()));

        engine.handle_change();

        assert_eq!(texts(&engine), vec!["copied".to_string()]);
    }

    #[test]
    fn empty_or_whitespace_text_is_not_captured() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());
        access.set_content(ClipboardContent::Text("  \r\n".into()));
        engine.handle_change();
        assert_eq!(texts(&engine), Vec::<String>::new());
    }

    #[test]
    fn non_text_image_content_is_ignored() {
        let access = FakeAccess::new();
        let engine = engine(access);
        // read() 返回 None（如文件列表、读取失败）：静默跳过
        engine.handle_change();
        assert_eq!(texts(&engine), Vec::<String>::new());
    }

    #[test]
    fn image_change_gets_captured() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());
        access.set_content(ClipboardContent::ImagePng(vec![1, 2, 3]));

        engine.handle_change();

        assert_eq!(texts(&engine), vec!["(image)".to_string()]);
    }

    #[test]
    fn own_write_text_is_excluded_from_history() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());

        engine.write_own_text("secret-password").unwrap();
        assert_eq!(
            access.writes(),
            vec![ClipboardContent::Text("secret-password".into())]
        );

        // 自家写入引发的多个变化事件序号相同，全部被过滤
        engine.handle_change();
        engine.handle_change();
        assert_eq!(texts(&engine), Vec::<String>::new());

        // 之后用户复制相同内容：序号已前进，正常入库
        access.set_content(ClipboardContent::Text("secret-password".into()));
        engine.handle_change();
        assert_eq!(texts(&engine), vec!["secret-password".to_string()]);
    }

    #[test]
    fn own_write_image_is_excluded_from_history() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());

        engine.write_own_image_png(&[7, 8, 9]).unwrap();
        engine.handle_change();
        assert_eq!(texts(&engine), Vec::<String>::new());
    }

    #[test]
    fn failed_own_write_does_not_exclude_content() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());
        access.set_fail_writes(true);

        assert!(engine.write_own_text("never written").is_err());

        // 写入失败：pending 撤销，后续变化正常入库
        access.set_fail_writes(false);
        access.set_content(ClipboardContent::Text("user content".into()));
        engine.handle_change();
        assert_eq!(texts(&engine), vec!["user content".to_string()]);
    }

    #[test]
    fn user_copy_same_content_after_own_write_is_captured() {
        let access = FakeAccess::new();
        let engine = engine(access.clone());

        engine.write_own_text("dup").unwrap();
        engine.handle_change();
        assert_eq!(texts(&engine), Vec::<String>::new());

        // 自家写入标记不消费：靠序号前进识别新的用户复制，同内容也入库
        access.set_content(ClipboardContent::Text("dup".into()));
        engine.handle_change();
        assert_eq!(texts(&engine), vec!["dup".to_string()]);
    }

    /// 端到端 smoke：真实剪贴板 → 真实引擎 → 真实 SQLite（变化通知由
    /// `handle_change` 手动触发，模拟监听线程回调；真实监听另有专门 smoke）。
    /// 不用真实监听线程的原因：测试进程退出前线程无法停止，泄漏的监听会与
    /// 其他 smoke 并发争用剪贴板。操作真实剪贴板，默认忽略，手动执行。
    #[test]
    #[ignore = "操作真实剪贴板，手动执行：cargo test -- --ignored --test-threads=1"]
    fn end_to_end_capture_and_own_write_channel() {
        use crate::platform::{ClipboardAccess as _, WinClipboardAccess};

        let dir = std::env::temp_dir().join(format!(
            "youmi-e2e-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        {
            let repo =
                Arc::new(ClipboardHistoryRepository::open(&dir.join("clipboard.db")).unwrap());
            let access: Arc<dyn ClipboardAccess> = Arc::new(WinClipboardAccess);
            let engine = ClipboardEngine::new(Arc::clone(&repo), access);

            // 1. 用户复制 → 入库
            WinClipboardAccess
                .write_text("youmi-e2e-user-copy")
                .unwrap();
            engine.handle_change();
            assert_eq!(texts(&engine), vec!["youmi-e2e-user-copy".to_string()]);

            // 2. 自家写入 → 不入历史（剪贴板内容变了，历史不变）
            engine.write_own_text("youmi-e2e-own-write").unwrap();
            engine.handle_change();
            assert_eq!(texts(&engine), vec!["youmi-e2e-user-copy".to_string()]);
            // 顺带确认自家写入确实到达了剪贴板
            assert_eq!(
                WinClipboardAccess.read(),
                Some(ClipboardContent::Text("youmi-e2e-own-write".into()))
            );

            // 3. 用户复制相同内容（序号已前进）→ 刷新原条目到顶，原条目仍在
            WinClipboardAccess
                .write_text("youmi-e2e-own-write")
                .unwrap();
            engine.handle_change();
            assert_eq!(
                texts(&engine),
                vec![
                    "youmi-e2e-own-write".to_string(),
                    "youmi-e2e-user-copy".to_string()
                ]
            );
        }
        // engine 与连接已 drop，Windows 上才能删除数据库文件
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
