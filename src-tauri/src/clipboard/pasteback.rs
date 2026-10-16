//! 回帖编排：浮层选中条目 → 条目写剪贴板（自家通道，不入史）→ 浮层隐藏、
//! 焦点交还呼出浮层前的窗口 → 模拟 Ctrl+V → 延迟恢复用户原剪贴板。
//!
//! - 目标窗口在热键呼起浮层时捕获（`capture_current_target`）：外部窗口与
//!   本进程主窗口（先唤起再粘贴）都可作目标；浮层窗口不作目标。
//! - 无有效目标（托盘直启、目标已关闭、浮层前台）或焦点/键击失败时降级为
//!   "仅复制"：条目留在剪贴板供手动粘贴，不模拟键击。
//! - 回帖成功后延迟恢复用户原剪贴板，恢复带序号守卫：延迟窗口内用户复制了
//!   新内容则放弃恢复，保护新内容。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::platform::PastebackAccess;

use super::engine::ClipboardEngine;
use super::model::ClipboardContent;
use super::repository::ClipboardHistoryRepository;

/// 焦点交还后等待目标窗口就绪再注入键击。
pub const FOCUS_SETTLE: Duration = Duration::from_millis(100);
/// 回帖后条目内容保留在剪贴板供目标窗口读取的时长，到点恢复用户原剪贴板。
pub const RESTORE_DELAY: Duration = Duration::from_millis(500);

/// 回帖目标：呼出浮层时的前台窗口（None = 无有效目标，回帖时降级为仅复制）。
pub struct PastebackState {
    pub target: Mutex<Option<isize>>,
    pub platform: Arc<dyn PastebackAccess>,
}

impl PastebackState {
    pub fn new(platform: Arc<dyn PastebackAccess>) -> Self {
        Self {
            target: Mutex::new(None),
            platform,
        }
    }
}

/// 回帖结果。`CopiedOnly` = 条目已写入剪贴板但未模拟粘贴（降级路径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PastebackOutcome {
    Pasted,
    CopiedOnly,
}

/// 执行一次回帖所需的协作对象（测试注入 fake 的接缝）。
pub struct PastebackDeps {
    pub repo: Arc<ClipboardHistoryRepository>,
    pub engine: Arc<ClipboardEngine>,
    pub input: Arc<dyn PastebackAccess>,
    /// 呼出浮层时捕获的目标窗口；None 走降级。
    pub target: Option<isize>,
    /// 本进程主窗口句柄；主窗口目标是合法回帖对象，回帖时先唤起。
    pub main_window: Option<isize>,
    pub own_pid: u32,
    pub focus_settle: Duration,
    pub restore_delay: Duration,
}

/// 执行一次回帖。`hide_overlay` 在浮层不再需要时调用（成功与降级路径都要隐藏）。
pub fn run(
    deps: &PastebackDeps,
    id: i64,
    hide_overlay: &dyn Fn(),
) -> Result<PastebackOutcome, String> {
    let content = deps
        .repo
        .entry_content(id)
        .map_err(|e| format!("读取条目 {id} 失败: {e}"))?
        .ok_or_else(|| format!("条目 {id} 不存在"))?;

    let clipboard = deps.engine.access().clone();
    let saved = clipboard.read();

    // 条目经自家写入通道写剪贴板：不新增、不刷新历史
    deps.engine
        .write_own(&content)
        .map_err(|e| format!("条目写入剪贴板失败: {e}"))?;
    let written_sequence = clipboard.sequence();

    let Some(target) = valid_target(deps) else {
        hide_overlay();
        // 降级：条目已就位于剪贴板供手动粘贴；没有发生粘贴，无需恢复原内容
        return Ok(PastebackOutcome::CopiedOnly);
    };

    hide_overlay();
    // 主窗口目标可能最小化或隐藏在托盘：先唤起（还原、显示、置前台）；
    // 外部窗口保持只置前台，不干扰其最小化/隐藏状态
    let foregrounded = if deps.main_window == Some(target) {
        deps.input.awaken_window(target)
    } else {
        deps.input.set_foreground(target)
    };
    if !foregrounded {
        eprintln!("youmi: 回帖聚焦目标窗口 {target:#x} 失败，降级为仅复制");
        return Ok(PastebackOutcome::CopiedOnly);
    }
    std::thread::sleep(deps.focus_settle);
    if deps.input.foreground_window() != Some(target) {
        eprintln!("youmi: 回帖目标窗口 {target:#x} 未获得前台焦点，降级为仅复制");
        return Ok(PastebackOutcome::CopiedOnly);
    }
    if let Err(e) = deps.input.send_paste() {
        eprintln!("youmi: 回帖模拟 Ctrl+V 失败，降级为仅复制: {e}");
        return Ok(PastebackOutcome::CopiedOnly);
    }

    schedule_restore(deps, clipboard, saved, written_sequence);
    Ok(PastebackOutcome::Pasted)
}

/// 目标有效性：窗口仍存在，且为外部窗口或本进程主窗口（浮层不是回帖对象）。
fn valid_target(deps: &PastebackDeps) -> Option<isize> {
    deps.target.filter(|hwnd| {
        deps.input
            .window_process(*hwnd)
            .is_some_and(|pid| pid != deps.own_pid || Some(*hwnd) == deps.main_window)
    })
}

/// 延迟恢复用户原剪贴板。序号守卫：剪贴板序号仍是回帖写入的那个，说明延迟
/// 窗口内没有任何新写入，恢复安全；否则（用户已复制新内容）放弃恢复。
/// 原内容不可读（如文件等未支持格式）或平台无序号时不盲目恢复。
fn schedule_restore(
    deps: &PastebackDeps,
    clipboard: Arc<dyn crate::platform::ClipboardAccess>,
    saved: Option<ClipboardContent>,
    written_sequence: Option<u64>,
) {
    let (Some(saved), Some(expected)) = (saved, written_sequence) else {
        return;
    };
    let engine = Arc::clone(&deps.engine);
    let delay = deps.restore_delay;
    if let Err(e) = std::thread::Builder::new()
        .name("pasteback-restore".into())
        .spawn(move || {
            std::thread::sleep(delay);
            if clipboard.sequence() != Some(expected) {
                return; // 窗口期内有新写入，不覆盖
            }
            if let Err(e) = engine.write_own(&saved) {
                eprintln!("youmi: 回帖恢复用户剪贴板失败: {e}");
            }
        })
    {
        eprintln!("youmi: 回帖恢复线程启动失败: {e}");
    }
}

/// 捕获当前前台窗口为回帖目标（热键呼起浮层时调用）。
/// 外部窗口与本进程主窗口记为目标；浮层窗口与其他自家窗口记为无目标。
pub fn capture_current_target(app: &AppHandle) {
    let Some(state) = app.try_state::<PastebackState>() else {
        return; // 剪贴板引擎未初始化等场景：无目标，回帖时自然降级
    };
    let main_window = main_window_hwnd(app);
    let target = capture_target(state.platform.as_ref(), std::process::id(), main_window);
    *state.target.lock().unwrap() = target;
}

/// 本进程主窗口句柄；窗口未创建（理论上不发生）时为 None。
pub fn main_window_hwnd(app: &AppHandle) -> Option<isize> {
    app.get_webview_window(crate::MAIN_WINDOW_LABEL)
        .and_then(|window| window.hwnd().ok())
        .map(|hwnd| hwnd.0 as isize)
}

/// 捕获逻辑核心：前台为外部窗口，或恰为本进程主窗口时作为目标；浮层前台
/// （含固定浮层）记为无目标，恰好清掉过期目标。
fn capture_target(
    input: &dyn PastebackAccess,
    own_pid: u32,
    main_window: Option<isize>,
) -> Option<isize> {
    let hwnd = input.foreground_window()?;
    if is_foreign_window(input, hwnd, own_pid) {
        return Some(hwnd);
    }
    // 自家窗口：仅主窗口可作回帖目标
    (Some(hwnd) == main_window).then_some(hwnd)
}

/// 窗口存活且不属于本进程：回帖目标与捕获共用的有效性谓词。
fn is_foreign_window(input: &dyn PastebackAccess, hwnd: isize, own_pid: u32) -> bool {
    input.window_process(hwnd).is_some_and(|pid| pid != own_pid)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::clipboard::repository::HistoryQuery;
    use crate::platform::{ClipboardAccess, PlatformError};

    // ---------- fakes ----------

    /// 假剪贴板：与 engine 测试同款语义——序号随每次内容变化递增。
    struct FakeClip(StdMutex<FakeClipState>);

    #[derive(Default)]
    struct FakeClipState {
        content: Option<ClipboardContent>,
        sequence: u64,
        writes: Vec<ClipboardContent>,
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

        fn writes(&self) -> Vec<ClipboardContent> {
            self.0.lock().unwrap().writes.clone()
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
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            s.writes.push(ClipboardContent::Text(text.into()));
            s.content = Some(ClipboardContent::Text(text.into()));
            Ok(())
        }

        fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
            let mut s = self.0.lock().unwrap();
            s.sequence += 1;
            s.writes.push(ClipboardContent::ImagePng(png.to_vec()));
            s.content = Some(ClipboardContent::ImagePng(png.to_vec()));
            Ok(())
        }
    }

    /// 假输入：记录 set_foreground/send_paste 调用，可编排前台行为。
    struct FakeInput(StdMutex<FakeInputState>);

    #[derive(Default)]
    struct FakeInputState {
        foreground: Option<isize>,
        /// hwnd → 进程 ID；不在表中 = 窗口已失效
        processes: HashMap<isize, u32>,
        /// set_foreground 是否成功并把前台交给目标
        focus_grants: bool,
        /// Some = set_foreground 成功后前台实际落点（模拟焦点被抢）
        foreground_after_focus: Option<Option<isize>>,
        fail_paste: bool,
        calls: Vec<Call>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        Focus(isize),
        Awaken(isize),
        Paste,
    }

    impl FakeInput {
        fn new(processes: HashMap<isize, u32>) -> Arc<Self> {
            Arc::new(Self(StdMutex::new(FakeInputState {
                processes,
                ..FakeInputState::default()
            })))
        }

        fn set_foreground_in(self: Arc<Self>, hwnd: isize) -> Arc<Self> {
            self.0.lock().unwrap().foreground = Some(hwnd);
            self
        }

        fn grant_focus(self: Arc<Self>) -> Arc<Self> {
            self.0.lock().unwrap().focus_grants = true;
            self
        }

        fn steal_focus_after(self: Arc<Self>, to: Option<isize>) -> Arc<Self> {
            self.0.lock().unwrap().foreground_after_focus = Some(to);
            self
        }

        fn fail_paste(self: Arc<Self>) -> Arc<Self> {
            self.0.lock().unwrap().fail_paste = true;
            self
        }

        fn calls(&self) -> Vec<Call> {
            self.0.lock().unwrap().calls.clone()
        }
    }

    impl PastebackAccess for FakeInput {
        fn foreground_window(&self) -> Option<isize> {
            self.0.lock().unwrap().foreground
        }

        fn window_process(&self, hwnd: isize) -> Option<u32> {
            self.0.lock().unwrap().processes.get(&hwnd).copied()
        }

        fn set_foreground(&self, hwnd: isize) -> bool {
            let mut s = self.0.lock().unwrap();
            s.calls.push(Call::Focus(hwnd));
            if s.focus_grants {
                s.foreground = s.foreground_after_focus.unwrap_or(Some(hwnd));
                true
            } else {
                false
            }
        }

        fn awaken_window(&self, hwnd: isize) -> bool {
            let mut s = self.0.lock().unwrap();
            s.calls.push(Call::Awaken(hwnd));
            if s.focus_grants {
                s.foreground = s.foreground_after_focus.unwrap_or(Some(hwnd));
                true
            } else {
                false
            }
        }

        fn send_paste(&self) -> Result<(), PlatformError> {
            let mut s = self.0.lock().unwrap();
            s.calls.push(Call::Paste);
            if s.fail_paste {
                Err(PlatformError("键击注入失败（测试注入）".into()))
            } else {
                Ok(())
            }
        }
    }

    // ---------- harness ----------

    const OWN_PID: u32 = 7;
    const TARGET: isize = 42;
    const FOREIGN_PID: u32 = 999;
    /// 本进程主窗口与浮层窗口（句柄为任意非零值，归属本进程）。
    const MAIN: isize = 100;
    const OVERLAY: isize = 200;

    struct Harness {
        repo: Arc<ClipboardHistoryRepository>,
        engine: Arc<ClipboardEngine>,
        clip: Arc<FakeClip>,
        input: Arc<FakeInput>,
        hides: Arc<StdMutex<usize>>,
    }

    fn harness(input: Arc<FakeInput>) -> Harness {
        let repo = Arc::new(ClipboardHistoryRepository::open_in_memory().unwrap());
        let clip = FakeClip::new();
        let engine = Arc::new(ClipboardEngine::new(
            Arc::clone(&repo),
            Arc::clone(&clip) as Arc<dyn ClipboardAccess>,
        ));
        Harness {
            repo,
            engine,
            clip,
            input,
            hides: Arc::new(StdMutex::new(0)),
        }
    }

    fn deps(h: &Harness, target: Option<isize>, restore_delay: Duration) -> PastebackDeps {
        deps_toward(h, target, None, restore_delay)
    }

    /// `main_window` = 主窗口句柄（测试主窗口目标路径时传入）。
    fn deps_toward(
        h: &Harness,
        target: Option<isize>,
        main_window: Option<isize>,
        restore_delay: Duration,
    ) -> PastebackDeps {
        PastebackDeps {
            repo: Arc::clone(&h.repo),
            engine: Arc::clone(&h.engine),
            input: Arc::clone(&h.input) as Arc<dyn PastebackAccess>,
            target,
            main_window,
            own_pid: OWN_PID,
            focus_settle: Duration::ZERO,
            restore_delay,
        }
    }

    fn run(h: &Harness, target: Option<isize>, id: i64) -> Result<PastebackOutcome, String> {
        run_with(h, target, id, Duration::ZERO)
    }

    fn run_with(
        h: &Harness,
        target: Option<isize>,
        id: i64,
        restore_delay: Duration,
    ) -> Result<PastebackOutcome, String> {
        let d = deps(h, target, restore_delay);
        let hides = Arc::clone(&h.hides);
        super::run(&d, id, &move || *hides.lock().unwrap() += 1)
    }

    /// 主窗口目标路径：deps 携带主窗口句柄。
    fn run_to_main(
        h: &Harness,
        target: Option<isize>,
        id: i64,
    ) -> Result<PastebackOutcome, String> {
        let d = deps_toward(h, target, Some(MAIN), Duration::ZERO);
        let hides = Arc::clone(&h.hides);
        super::run(&d, id, &move || *hides.lock().unwrap() += 1)
    }

    fn history_texts(h: &Harness) -> Vec<String> {
        h.repo
            .query(&HistoryQuery::default())
            .unwrap()
            .into_iter()
            .map(|e| e.text.unwrap_or_else(|| "(image)".into()))
            .collect()
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

    // ---------- tests ----------

    #[test]
    fn pasteback_pastes_to_target_and_restores_user_clipboard() {
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]))
            .set_foreground_in(TARGET)
            .grant_focus();
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::Pasted);
        // 浮层隐藏恰好一次；焦点交还目标后模拟粘贴
        assert_eq!(*h.hides.lock().unwrap(), 1);
        assert_eq!(h.input.calls(), vec![Call::Focus(TARGET), Call::Paste]);
        // 条目经自家通道写入剪贴板
        assert!(h
            .clip
            .writes()
            .contains(&ClipboardContent::Text("hello".into())));
        // 历史状态不变：条目不刷新、不新增
        assert_eq!(history_texts(&h), vec!["hello".to_string()]);
        // 延迟后用户原剪贴板恢复
        let restored = wait_until(Duration::from_secs(2), || {
            h.clip.content() == Some(ClipboardContent::Text("user-content".into()))
        });
        assert!(restored, "回帖后应恢复用户原剪贴板内容");
    }

    #[test]
    fn pasteback_image_entry_pastes_png() {
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]))
            .set_foreground_in(TARGET)
            .grant_focus();
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::ImagePng(vec![9, 9, 9]))
            .unwrap();

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::Pasted);
        assert!(h
            .clip
            .writes()
            .contains(&ClipboardContent::ImagePng(vec![9, 9, 9])));
        assert_eq!(h.input.calls(), vec![Call::Focus(TARGET), Call::Paste]);
    }

    #[test]
    fn pasteback_without_target_degrades_to_copy_only() {
        let input = FakeInput::new(HashMap::new());
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));

        let outcome = run(&h, None, 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert_eq!(*h.hides.lock().unwrap(), 1);
        assert!(h.input.calls().is_empty());
        // 降级语义：条目已就位于剪贴板供手动粘贴
        assert!(h
            .clip
            .writes()
            .contains(&ClipboardContent::Text("hello".into())));
        // 没有发生粘贴：不恢复原内容（条目需要留在剪贴板）
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("hello".into()))
        );
    }

    #[test]
    fn pasteback_to_own_overlay_window_degrades_to_copy_only() {
        // 浮层窗口即使存活也不作回帖目标（主窗口已知的情况下）
        let input = FakeInput::new(HashMap::from([(OVERLAY, OWN_PID)]));
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run_to_main(&h, Some(OVERLAY), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert!(h.input.calls().is_empty());
    }

    #[test]
    fn pasteback_pastes_to_main_window_after_awakening() {
        // 焦点本就在主窗口（呼出浮层前的前台是主窗口输入框）
        let input = FakeInput::new(HashMap::from([(MAIN, OWN_PID)]))
            .set_foreground_in(MAIN)
            .grant_focus();
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));

        let outcome = run_to_main(&h, Some(MAIN), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::Pasted);
        // 先唤起（还原/显示/置前台）再粘贴，不走外部窗口的 set_foreground
        assert_eq!(h.input.calls(), vec![Call::Awaken(MAIN), Call::Paste]);
        assert!(h
            .clip
            .writes()
            .contains(&ClipboardContent::Text("hello".into())));
        assert_eq!(history_texts(&h), vec!["hello".to_string()]);
        let restored = wait_until(Duration::from_secs(2), || {
            h.clip.content() == Some(ClipboardContent::Text("user-content".into()))
        });
        assert!(restored, "主窗口回帖后应恢复用户原剪贴板内容");
    }

    #[test]
    fn pasteback_awaken_failure_degrades_to_copy_only() {
        // 唤起失败（如前台锁）：不注入粘贴
        let input = FakeInput::new(HashMap::from([(MAIN, OWN_PID)]));
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run_to_main(&h, Some(MAIN), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert_eq!(h.input.calls(), vec![Call::Awaken(MAIN)]);
    }

    #[test]
    fn pasteback_to_closed_main_window_degrades_to_copy_only() {
        // 记录的主窗口句柄已失效（进程归属解析不出）：照常降级
        let input = FakeInput::new(HashMap::new()).set_foreground_in(MAIN);
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run_to_main(&h, Some(MAIN), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert!(h.input.calls().is_empty());
    }

    #[test]
    fn pasteback_to_closed_window_degrades_to_copy_only() {
        // 目标窗口已关闭：进程归属解析不出来
        let input = FakeInput::new(HashMap::new()).set_foreground_in(TARGET);
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert!(h.input.calls().is_empty());
    }

    #[test]
    fn pasteback_focus_failure_degrades_to_copy_only() {
        // set_foreground 失败（如前台锁）
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]));
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert_eq!(h.input.calls(), vec![Call::Focus(TARGET)]);
        assert!(!h.input.calls().contains(&Call::Paste));
    }

    #[test]
    fn pasteback_focus_verify_failure_degrades_to_copy_only() {
        // set_foreground 返回成功但前台被别的窗口抢走
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]))
            .set_foreground_in(TARGET)
            .grant_focus()
            .steal_focus_after(Some(77));
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert!(!h.input.calls().contains(&Call::Paste));
    }

    #[test]
    fn pasteback_send_paste_failure_degrades_to_copy_only() {
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]))
            .set_foreground_in(TARGET)
            .grant_focus()
            .fail_paste();
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();

        let outcome = run(&h, Some(TARGET), 1).unwrap();

        assert_eq!(outcome, PastebackOutcome::CopiedOnly);
        assert_eq!(h.input.calls(), vec![Call::Focus(TARGET), Call::Paste]);
    }

    #[test]
    fn pasteback_restore_skipped_when_user_copies_new_content() {
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]))
            .set_foreground_in(TARGET)
            .grant_focus();
        let h = harness(input);
        h.repo
            .capture(&ClipboardContent::Text("hello".into()))
            .unwrap();
        h.clip
            .set_content(ClipboardContent::Text("user-content".into()));

        let outcome = run_with(&h, Some(TARGET), 1, Duration::from_millis(80)).unwrap();
        assert_eq!(outcome, PastebackOutcome::Pasted);

        // 恢复窗口期内用户复制了新内容：恢复放弃，新内容不被覆盖
        h.clip
            .set_content(ClipboardContent::Text("fresh-copy".into()));
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            h.clip.content(),
            Some(ClipboardContent::Text("fresh-copy".into()))
        );
    }

    #[test]
    fn pasteback_missing_entry_is_error() {
        let input = FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)]));
        let h = harness(input);

        let outcome = run(&h, Some(TARGET), 12345);

        assert!(outcome.is_err());
        assert_eq!(*h.hides.lock().unwrap(), 0);
    }

    #[test]
    fn capture_target_takes_foreign_foreground() {
        let input =
            FakeInput::new(HashMap::from([(TARGET, FOREIGN_PID)])).set_foreground_in(TARGET);
        assert_eq!(
            super::capture_target(input.as_ref(), OWN_PID, Some(MAIN)),
            Some(TARGET)
        );
    }

    #[test]
    fn capture_target_takes_main_window_foreground() {
        let input = FakeInput::new(HashMap::from([(MAIN, OWN_PID)])).set_foreground_in(MAIN);
        assert_eq!(
            super::capture_target(input.as_ref(), OWN_PID, Some(MAIN)),
            Some(MAIN)
        );
    }

    #[test]
    fn capture_target_rejects_overlay_windows() {
        // 浮层前台（含固定浮层）不作目标：记为无目标，恰好清掉过期目标
        let input = FakeInput::new(HashMap::from([(OVERLAY, OWN_PID)])).set_foreground_in(OVERLAY);
        assert_eq!(
            super::capture_target(input.as_ref(), OWN_PID, Some(MAIN)),
            None
        );
    }

    #[test]
    fn capture_target_rejects_own_window_without_main() {
        // 主窗口句柄取不到（窗口未创建等）：自家窗口一律不作目标
        let input = FakeInput::new(HashMap::from([(MAIN, OWN_PID)])).set_foreground_in(MAIN);
        assert_eq!(super::capture_target(input.as_ref(), OWN_PID, None), None);
    }

    #[test]
    fn capture_target_none_without_foreground() {
        let input = FakeInput::new(HashMap::new());
        assert_eq!(
            super::capture_target(input.as_ref(), OWN_PID, Some(MAIN)),
            None
        );
    }

    #[test]
    fn capture_target_rejects_dead_window() {
        let input = FakeInput::new(HashMap::new()).set_foreground_in(TARGET);
        assert_eq!(
            super::capture_target(input.as_ref(), OWN_PID, Some(MAIN)),
            None
        );
    }
}
