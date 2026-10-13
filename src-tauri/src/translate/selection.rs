//! 划词取词编排：向前台应用模拟 Ctrl+C → 等待剪贴板变化 → 读出选中文本 →
//! 豁免历史入库 → 恢复用户原剪贴板。
//!
//! - 自家窗口聚焦时不注入键击：Ctrl+C 会落进 Youmi 自己的界面。
//! - 模拟复制由目标应用代写剪贴板，经引擎的模拟复制过滤窗口（与自家写入
//!   同一套序号识别）豁免入库，历史与用户剪贴板均不被污染。
//! - 原剪贴板不可读（文件列表等未支持格式）时无法恢复，选中文本留在剪贴板：
//!   未支持格式没有通用恢复手段，宁留勿毁。

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::clipboard::engine::ClipboardEngine;
use crate::clipboard::model::ClipboardContent;
use crate::platform::SelectionAccess;

/// 注入 Ctrl+C 后等待目标应用响应的首轮延迟（常规应用处理 WM_COPY 远快于此）。
pub const SETTLE: Duration = Duration::from_millis(30);
/// 等待剪贴板变化的总时长上限：超时按"无选中文本"处理。
pub const TIMEOUT: Duration = Duration::from_millis(600);
/// 轮询剪贴板序号的间隔。
pub const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// 取词结果。`Nothing` 含无选中文本、超时未变化、变化后为空白/非文本等场景。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Text(String),
    Nothing,
}

/// 执行一次取词所需的协作对象（测试注入 fake 的接缝）。
pub struct SelectionDeps {
    pub engine: Arc<ClipboardEngine>,
    pub input: Arc<dyn SelectionAccess>,
    pub own_pid: u32,
    pub settle: Duration,
    pub timeout: Duration,
    pub poll_interval: Duration,
}

/// 执行一次取词。
pub fn capture(deps: &SelectionDeps) -> Result<Selection, String> {
    // 自家窗口（主窗口/浮层）聚焦时不注入
    if deps
        .input
        .foreground_process()
        .is_none_or(|pid| pid == deps.own_pid)
    {
        return Ok(Selection::Nothing);
    }

    let clipboard = deps.engine.access().clone();
    let saved = clipboard.read();
    let baseline = clipboard.sequence();

    deps.engine.begin_simulated_copy();
    if let Err(e) = deps.input.send_copy() {
        deps.engine.abort_simulated_copy();
        return Err(format!("模拟复制失败: {e}"));
    }

    // 轮询等待目标应用代写：序号相对基线前进才算变化（平台无序号时首读即取）
    let mut observed = None;
    std::thread::sleep(deps.settle);
    let deadline = Instant::now() + deps.timeout;
    loop {
        if let Some(sequence) = clipboard
            .sequence()
            .filter(|sequence| Some(*sequence) != baseline)
        {
            observed = Some(sequence);
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(deps.poll_interval);
    }

    let Some(observed) = observed else {
        // 应用没有代写（无选中文本/不支持复制）：剪贴板未被改动，无需恢复
        deps.engine.abort_simulated_copy();
        return Ok(Selection::Nothing);
    };

    // 代写引发的监听变化事件按该序号过滤，不入历史
    deps.engine.finish_simulated_copy();

    let selection = match clipboard.read() {
        Some(ClipboardContent::Text(text)) if !text.trim().is_empty() => Selection::Text(text),
        _ => Selection::Nothing, // 代写结果为空白/非文本（应用复制失败等）
    };
    restore(&clipboard, deps.engine.as_ref(), saved, observed);
    Ok(selection)
}

/// 恢复用户原剪贴板。序号守卫：剪贴板仍是取词写入的那个序号才恢复，读词与
/// 恢复之间用户复制了新内容则放弃（保护新内容）；原内容不可读时无法恢复。
fn restore(
    clipboard: &Arc<dyn crate::platform::ClipboardAccess>,
    engine: &ClipboardEngine,
    saved: Option<ClipboardContent>,
    observed: u64,
) {
    let Some(saved) = saved else {
        return;
    };
    if clipboard.sequence() != Some(observed) {
        return;
    }
    if let Err(e) = engine.write_own(&saved) {
        eprintln!("youmi: 划词取词恢复用户剪贴板失败: {e}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    use super::*;
    use crate::clipboard::repository::{ClipboardHistoryRepository, HistoryQuery};
    use crate::platform::{ClipboardAccess, PlatformError};

    // ---------- fakes ----------

    /// 假剪贴板：序号随每次内容变化递增，与真实剪贴板语义一致。
    struct FakeClip(StdMutex<FakeClipState>);

    #[derive(Default)]
    struct FakeClipState {
        content: Option<ClipboardContent>,
        sequence: u64,
    }

    impl FakeClip {
        fn new() -> Arc<Self> {
            Arc::new(Self(StdMutex::new(FakeClipState::default())))
        }

        fn set_content(&self, content: ClipboardContent) {
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            s.content = Some(content);
        }

        fn content(&self) -> Option<ClipboardContent> {
            self.0.lock().unwrap().content.clone()
        }
    }

    impl ClipboardAccess for FakeClip {
        fn read(&self) -> Option<ClipboardContent> {
            self.0.lock().unwrap().content.clone()
        }

        fn sequence(&self) -> Option<u64> {
            Some(self.0.lock().unwrap().sequence)
        }

        fn write_text(&self, text: &str) -> Result<(), PlatformError> {
            self.set_content(ClipboardContent::Text(text.into()));
            Ok(())
        }

        fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
            self.set_content(ClipboardContent::ImagePng(png.to_vec()));
            Ok(())
        }
    }

    /// 假输入：记录 send_copy 调用；可配置前台进程归属、注入失败、
    /// 以及 send_copy 后按脚本延迟代写剪贴板（模拟目标应用响应 Ctrl+C）。
    struct FakeInput {
        clip: Arc<FakeClip>,
        state: StdMutex<FakeInputState>,
    }

    #[derive(Default)]
    struct FakeInputState {
        foreground_pid: Option<u32>,
        fail_copy: bool,
        copy_calls: usize,
        script: Option<(Duration, ClipboardContent)>,
    }

    impl FakeInput {
        fn new(clip: Arc<FakeClip>, foreground_pid: Option<u32>) -> Arc<Self> {
            Arc::new(Self {
                clip,
                state: StdMutex::new(FakeInputState {
                    foreground_pid,
                    ..FakeInputState::default()
                }),
            })
        }

        fn responds_with(&self, delay: Duration, content: ClipboardContent) {
            self.state.lock().unwrap().script = Some((delay, content));
        }

        fn fail_copy(&self) {
            self.state.lock().unwrap().fail_copy = true;
        }

        fn copy_calls(&self) -> usize {
            self.state.lock().unwrap().copy_calls
        }
    }

    impl SelectionAccess for FakeInput {
        fn foreground_process(&self) -> Option<u32> {
            self.state.lock().unwrap().foreground_pid
        }

        fn send_copy(&self) -> Result<(), PlatformError> {
            let mut s = self.state.lock().unwrap();
            s.copy_calls += 1;
            if s.fail_copy {
                return Err(PlatformError("键击注入失败（测试注入）".into()));
            }
            let script = s.script.clone();
            drop(s);
            if let Some((delay, content)) = script {
                let clip = Arc::clone(&self.clip);
                // 模拟目标应用响应 Ctrl+C 后代写剪贴板
                std::thread::spawn(move || {
                    std::thread::sleep(delay);
                    clip.set_content(content);
                });
            }
            Ok(())
        }
    }

    // ---------- harness ----------

    const OWN_PID: u32 = 7;
    const FOREIGN_PID: u32 = 999;

    struct Harness {
        repo: Arc<ClipboardHistoryRepository>,
        engine: Arc<ClipboardEngine>,
        clip: Arc<FakeClip>,
        input: Arc<FakeInput>,
    }

    fn harness(foreground_pid: Option<u32>) -> Harness {
        let repo = Arc::new(ClipboardHistoryRepository::open_in_memory().unwrap());
        let clip = FakeClip::new();
        let engine = Arc::new(ClipboardEngine::new(
            Arc::clone(&repo),
            Arc::clone(&clip) as Arc<dyn ClipboardAccess>,
        ));
        let input = FakeInput::new(Arc::clone(&clip), foreground_pid);
        Harness {
            repo,
            engine,
            clip,
            input,
        }
    }

    fn deps(h: &Harness) -> SelectionDeps {
        SelectionDeps {
            engine: Arc::clone(&h.engine),
            input: Arc::clone(&h.input) as Arc<dyn SelectionAccess>,
            own_pid: OWN_PID,
            settle: Duration::from_millis(2),
            timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(2),
        }
    }

    fn history_texts(h: &Harness) -> Vec<String> {
        h.repo
            .query(&HistoryQuery::default())
            .unwrap()
            .into_iter()
            .map(|e| e.text.unwrap_or_else(|| "(image)".into()))
            .collect()
    }

    // ---------- tests ----------

    #[test]
    fn capture_reads_selection_and_restores_user_clipboard() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));
        h.input.responds_with(
            Duration::from_millis(10),
            ClipboardContent::Text("selected text".into()),
        );

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Text("selected text".into()));
        assert_eq!(h.input.copy_calls(), 1);
        // 恢复在 capture 内同步完成：返回时用户原剪贴板已就位
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("user-content".into()))
        );
    }

    #[test]
    fn simulated_copy_never_enters_history() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));
        h.input.responds_with(
            Duration::from_millis(10),
            ClipboardContent::Text("selected text".into()),
        );

        capture(&deps(&h)).unwrap();

        // 代写引发的监听变化事件被序号过滤，历史保持为空
        h.engine.handle_change();
        h.engine.handle_change();
        assert_eq!(history_texts(&h), Vec::<String>::new());

        // 之后用户复制相同内容：序号已前进，正常入库
        h.clip
            .set_content(ClipboardContent::Text("selected text".into()));
        h.engine.handle_change();
        assert_eq!(history_texts(&h), vec!["selected text".to_string()]);
    }

    #[test]
    fn capture_skips_own_process_foreground() {
        let h = harness(Some(OWN_PID));

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Nothing);
        assert_eq!(h.input.copy_calls(), 0);
    }

    #[test]
    fn capture_skips_when_foreground_unknown() {
        let h = harness(None);

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Nothing);
        assert_eq!(h.input.copy_calls(), 0);
    }

    #[test]
    fn capture_times_out_without_response() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Nothing);
        assert_eq!(h.input.copy_calls(), 1);
        // 应用没有代写：剪贴板未被改动
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("user-content".into()))
        );
        // 过滤窗口已撤销：迟到的代写与用户复制都正常入库
        h.clip
            .set_content(ClipboardContent::Text("late copy".into()));
        h.engine.handle_change();
        assert_eq!(history_texts(&h), vec!["late copy".to_string()]);
    }

    #[test]
    fn capture_send_copy_failure_is_error() {
        let h = harness(Some(FOREIGN_PID));
        h.input.fail_copy();

        let outcome = capture(&deps(&h));

        assert!(outcome.is_err());
        // 过滤窗口已撤销：后续用户复制正常入库
        h.clip
            .set_content(ClipboardContent::Text("user copy".into()));
        h.engine.handle_change();
        assert_eq!(history_texts(&h), vec!["user copy".to_string()]);
    }

    #[test]
    fn capture_restores_image_clipboard() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::ImagePng(vec![1, 2, 3]));
        h.input.responds_with(
            Duration::from_millis(10),
            ClipboardContent::Text("selected text".into()),
        );

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Text("selected text".into()));
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::ImagePng(vec![1, 2, 3]))
        );
    }

    #[test]
    fn capture_keeps_selection_when_original_unreadable() {
        // 原剪贴板为空（不可读）：无法恢复，选中文本留在剪贴板
        let h = harness(Some(FOREIGN_PID));
        h.input.responds_with(
            Duration::from_millis(10),
            ClipboardContent::Text("selected text".into()),
        );

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Text("selected text".into()));
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("selected text".into()))
        );
    }

    #[test]
    fn capture_blank_response_is_nothing() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));
        // 应用响应了但写入空白（复制失败等）：照常豁免并恢复
        h.input.responds_with(
            Duration::from_millis(10),
            ClipboardContent::Text("   ".into()),
        );

        let outcome = capture(&deps(&h)).unwrap();

        assert_eq!(outcome, Selection::Nothing);
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("user-content".into()))
        );
        h.engine.handle_change();
        assert_eq!(history_texts(&h), Vec::<String>::new());
    }

    #[test]
    fn restore_skips_when_user_copied_new_content() {
        let h = harness(Some(FOREIGN_PID));
        h.clip
            .set_content(ClipboardContent::Text("stale-observed".into()));
        let observed = h.clip.sequence().unwrap();
        // 读词与恢复之间用户复制了新内容
        h.clip
            .set_content(ClipboardContent::Text("fresh copy".into()));

        restore(
            &(Arc::clone(&h.clip) as Arc<dyn ClipboardAccess>),
            h.engine.as_ref(),
            Some(ClipboardContent::Text("user-content".into())),
            observed,
        );

        // 新内容不被覆盖
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("fresh copy".into()))
        );
    }
}
