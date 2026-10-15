//! Win32 后端：组合键录制（ADR-0001 快捷键录制 seam）。
//!
//! 录制期经 WH_KEYBOARD_LL 低级钩子截获按键并整体吞掉：Alt+Space（窗口系统
//! 菜单）、Win+E（Shell）这类 OS 保留组合到不了 webview 的 keydown，DOM 层
//! 捕获对它们天然失效。会话为一次性：截获第一个非修饰键组合、Esc 取消、
//! 超时自动取消。
//!
//! 低级钩子按「后装先叫」排队：其他软件在会话开始后重挂键盘钩子会排到前
//! 面把按键截走（实测本机存在此类动态钩子），钩子收不到事件。因此录制期并
//! 行跑一个异步键表轮询兜底（见 `poll_thread_loop`），按下沿截获不依赖钩子
//! 链位次；钩子在前时先截获并吞键，轮询自然沉寂。端到端行为需真实键击（不
//! 做单测），VK 映射、代数状态机与按下沿扫描为纯函数单测覆盖。

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_ADD, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DECIMAL, VK_DELETE, VK_DIVIDE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK, VK_NUMPAD0, VK_NUMPAD9,
    VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA,
    VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB,
    VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PeekMessageW, PostThreadMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS, LLKHF_EXTENDED, MSG, PM_NOREMOVE,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::platform::{RecordOutcome, ShortcutRecorder};

use super::key_is_down;

/// 会话超时：期间未截获到组合键自动取消，吞按键的时间窗有上界。
const TIMEOUT: Duration = Duration::from_secs(10);

/// 轮询兜底采样间隔：远短于人手按键时长，按下沿不会滑过采样点。
const POLL_INTERVAL: Duration = Duration::from_millis(15);

/// 录制会话：录制命令、钩子回调、看门狗与钩子线程共享。
/// outcome 由钩子/看门狗/取消写入；钩子线程退出消息循环后统一回调 sink，
/// emit 不发生在钩子回调内（低级钩子回调必须快，超时会被系统摘除钩子）。
struct Session {
    generation: u64,
    active: bool,
    started: Instant,
    /// 钩子线程 id；0 = 线程尚未就绪（安装钩子前）。
    thread_id: u32,
    /// 修饰键按下态：由钩子事件跟踪（见 [`Mods`]）。
    mods: Mods,
    outcome: Option<RecordOutcome>,
    sink: Arc<dyn Fn(RecordOutcome) + Send + Sync>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// 会话代数，全局单调递增：不复用代数，迟到的旧看门狗才不可能误杀新会话。
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

// 本钩子安装所属的会话代数（钩子回调运行在安装线程上）。
thread_local! {
    static HOOK_GENERATION: Cell<u64> = const { Cell::new(0) };
}

/// 录制期修饰键按下态。被低级钩子吞掉的按键不会进入系统输入管线，异步键表
/// （GetAsyncKeyState）对它们不更新，修饰键态只能由钩子事件自己维护；
/// 仅录制前已按下的修饰键以开始时刻的异步键表为种子。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Mods {
    ctrl: bool,
    alt: bool,
    shift: bool,
    super_key: bool,
}

impl Mods {
    fn seeded() -> Self {
        Self::seeded_with(key_is_down)
    }

    /// `seeded` 的可测变体：键态查询函数可替换（单测无需真实按下修饰键）。
    fn seeded_with(is_down: fn(VIRTUAL_KEY) -> bool) -> Self {
        Self {
            ctrl: is_down(VK_CONTROL),
            alt: is_down(VK_MENU),
            shift: is_down(VK_SHIFT),
            super_key: is_down(VK_LWIN) || is_down(VK_RWIN),
        }
    }

    /// 用一个钩子按键事件更新修饰键态；非修饰键为空操作。
    fn absorb(&mut self, vk: u32, is_down: bool) {
        let slot = match VIRTUAL_KEY(vk as u16) {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => &mut self.ctrl,
            VK_MENU | VK_LMENU | VK_RMENU => &mut self.alt,
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => &mut self.shift,
            VK_LWIN | VK_RWIN => &mut self.super_key,
            _ => return,
        };
        *slot = is_down;
    }

    /// 规范组合键：修饰键序 ctrl, alt, shift, super，主键为
    /// KeyboardEvent.code 命名，与动作绑定的格式一致。
    fn combo_string(&self, code: &str) -> String {
        let mut combo = String::new();
        if self.ctrl {
            combo.push_str("ctrl+");
        }
        if self.alt {
            combo.push_str("alt+");
        }
        if self.shift {
            combo.push_str("shift+");
        }
        if self.super_key {
            combo.push_str("super+");
        }
        combo.push_str(code);
        combo
    }
}

pub struct WinShortcutRecorder;

impl ShortcutRecorder for WinShortcutRecorder {
    fn begin(&self, sink: Arc<dyn Fn(RecordOutcome) + Send + Sync>) -> Result<(), String> {
        let mut session = SESSION.lock().unwrap();
        let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        let stale_thread_id = session
            .as_ref()
            .filter(|s| s.active)
            .map(|s| s.thread_id)
            .unwrap_or(0);
        *session = Some(Session {
            generation,
            active: true,
            started: Instant::now(),
            thread_id: 0,
            mods: Mods::seeded(),
            outcome: None,
            sink,
        });
        drop(session);
        // 接管已有会话：旧钩子按代数失配放行按键，旧线程退出且不重复回调
        if stale_thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(stale_thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }

        std::thread::Builder::new()
            .name("shortcut-record-hook".into())
            .spawn(move || hook_thread_loop(generation))
            .map_err(|e| {
                // 钩子线程起不来：清掉会话，不让吞按键状态悬挂
                let mut session = SESSION.lock().unwrap();
                if session.as_ref().is_some_and(|s| s.generation == generation) {
                    *session = None;
                }
                e.to_string()
            })?;

        std::thread::Builder::new()
            .name("shortcut-record-poll".into())
            .spawn(move || poll_thread_loop(generation))
            .map_err(|e| {
                let mut session = SESSION.lock().unwrap();
                if session.as_ref().is_some_and(|s| s.generation == generation) {
                    *session = None;
                }
                e.to_string()
            })?;

        std::thread::Builder::new()
            .name("shortcut-record-watchdog".into())
            .spawn(move || {
                std::thread::sleep(TIMEOUT);
                stop_session(generation, RecordOutcome::Timeout);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn cancel(&self) {
        stop_session(0, RecordOutcome::Cancelled);
    }
}

/// 结束会话（`generation` 为 0 时不限代数）：置为给定 outcome 并通知钩子线程收尾。
fn stop_session(generation: u64, outcome: RecordOutcome) {
    let thread_id = inactivate(generation, outcome);
    notify_hook_thread(thread_id);
}

/// 结束同代活跃会话（`generation` 为 0 时不限代数）：置为给定 outcome（仅
/// 第一个到达者生效）。返回会话的钩子线程 id（0 = 未就绪）。
fn inactivate(generation: u64, outcome: RecordOutcome) -> u32 {
    let mut session = SESSION.lock().unwrap();
    let Some(s) = session.as_mut() else {
        return 0;
    };
    if generation != 0 && s.generation != generation {
        return 0;
    }
    if !s.active {
        return 0;
    }
    s.active = false;
    s.outcome = Some(outcome);
    s.thread_id
}

/// 钩子线程：装钩子 → 消息循环（WM_QUIT 退出）→ 摘钩子 → 交付结果。
fn hook_thread_loop(generation: u64) {
    // 先创建本线程消息队列再发布线程 id：PostThreadMessageW 对无队列线程会失败
    let mut msg = MSG::default();
    let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE) };
    let thread_id = unsafe { GetCurrentThreadId() };

    let install = {
        let mut session = SESSION.lock().unwrap();
        let Some(s) = session.as_mut() else {
            return;
        };
        if s.generation != generation {
            // 已被新会话接管：本线程退出且不触碰新会话
            return;
        }
        if s.active {
            s.thread_id = thread_id;
        }
        s.active
    };
    if !install {
        // cancel 先于线程就绪：把已写入的 Cancelled outcome 交付出去
        finish_session(generation);
        return;
    }
    let Ok(hook) =
        (unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) })
    else {
        eprintln!("youmi: 安装键盘录制钩子失败");
        finish_session(generation);
        return;
    };
    HOOK_GENERATION.with(|g| g.set(generation));
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {}
    let _ = unsafe { UnhookWindowsHookEx(hook) };
    finish_session(generation);
}

/// 交付会话结果给 sink；只有同代的钩子线程调用（outcome 缺省为 Cancelled）。
fn finish_session(generation: u64) {
    let sink = {
        let mut session = SESSION.lock().unwrap();
        if !session.as_ref().is_some_and(|s| s.generation == generation) {
            return;
        }
        session.take().map(|s| (s.sink, s.outcome))
    };
    if let Some((sink, outcome)) = sink {
        sink(outcome.unwrap_or(RecordOutcome::Cancelled));
    }
}

/// 通知钩子线程收尾（摘钩子 → 交付）；0 = 线程未就绪，无需通知。
fn notify_hook_thread(thread_id: u32) {
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// 轮询兜底线程：低级钩子按「后装先叫」排队，其他软件若在会话开始后重挂
/// 键盘钩子会排到我们前面把按键截走——钩子收不到事件，但按键已照常流入系
/// 统（异步键表更新）。本线程每 [`POLL_INTERVAL`] 采样一次异步键表，按下沿
/// 兜底截获。钩子在前时它先截获并吞键，异步键表不更新，轮询采不到按下沿，
/// 自然沉寂；两路以会话 active 位先到先得，不会双报。
fn poll_thread_loop(generation: u64) {
    let mut watch = EdgeWatch::default();
    loop {
        std::thread::sleep(POLL_INTERVAL);
        let hit = {
            let mut session = SESSION.lock().unwrap();
            let Some(s) = session.as_mut() else {
                return;
            };
            if !s.active || s.generation != generation {
                return;
            }
            // 轮询无法区分扩展位（小键盘回车按主 Enter 记），采样成本可忽略
            scan_press_edges(&mut watch, &s.mods, |vk| {
                key_is_down(VIRTUAL_KEY(vk as u16))
            })
            .map(|(vk, mods)| (poll_outcome(vk, &mods), s.thread_id))
        };
        let Some((outcome, thread_id)) = hit else {
            continue;
        };
        // 置 outcome（先到先得，钩子并发截获时其 outcome 优先）并通知收尾
        inactivate(generation, outcome);
        notify_hook_thread(thread_id);
        finish_session(generation);
        return;
    }
}

/// 轮询截获的 outcome：Esc → Cancelled；可映射键 → Recorded；不可映射的
/// 杂键（IME 中间态等）与钩子路径同语义，按 Cancelled 收场。
fn poll_outcome(vk: u32, mods: &Mods) -> RecordOutcome {
    if vk == VK_ESCAPE.0 as u32 {
        return RecordOutcome::Cancelled;
    }
    match vk_to_code(vk, false) {
        Some(code) => RecordOutcome::Recorded(mods.combo_string(code)),
        None => RecordOutcome::Cancelled,
    }
}

/// 一轮按下沿扫描：`read` 采样各键当前按下态，返回会话内新按下的首个非修
/// 饰键 `Some((vk, 修饰键态))`。首个 tick 只建基线不报沿（录制开始前已按住
/// 的键不算新按下）。修饰键态 = 钩子跟踪态 ∪ 异步键表当前态：被钩子吞掉的
/// 按键不进异步键表、只有跟踪态知道；被抢位钩子截走的键只有异步键表知道。
fn scan_press_edges(
    watch: &mut EdgeWatch,
    tracked: &Mods,
    read: impl Fn(u32) -> bool,
) -> Option<(u32, Mods)> {
    let mut now = [false; 256];
    let mut hit = None;
    for vk in 0x01u32..=0xFF {
        let down = read(vk);
        now[vk as usize] = down;
        if down && watch.seeded && !watch.prev[vk as usize] && !is_modifier(vk) && hit.is_none() {
            hit = Some(vk);
        }
    }
    if !watch.seeded {
        // 首轮建基线：此刻已按下的键不算录制期内的新按下
        watch.prev = now;
        watch.seeded = true;
        return None;
    }
    watch.prev = now;
    let mods = {
        let mut merged = *tracked;
        merged.ctrl |= now[VK_CONTROL.0 as usize];
        merged.alt |= now[VK_MENU.0 as usize];
        merged.shift |= now[VK_SHIFT.0 as usize];
        merged.super_key |= now[VK_LWIN.0 as usize] || now[VK_RWIN.0 as usize];
        merged
    };
    hit.map(|vk| (vk, mods))
}

/// 跨 tick 的键态快照：`prev` 为上一轮按下态，`seeded` 表示基线已建。
struct EdgeWatch {
    prev: [bool; 256],
    seeded: bool,
}

impl Default for EdgeWatch {
    fn default() -> Self {
        Self {
            prev: [false; 256],
            seeded: false,
        }
    }
}

/// 低级键盘钩子：录制会话活跃期间吞掉全部按键（不让系统菜单/Shell 等对
/// 按键产生反应），非修饰键按下即完成截获。回调必须快，不做 emit。
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let generation = HOOK_GENERATION.with(|g| g.get());
        let decision = {
            let mut session = SESSION.lock().unwrap();
            match session.as_mut() {
                Some(s) if s.active && s.generation == generation => {
                    capture_if_recording(s, wparam, lparam)
                }
                _ => Capture::PassThrough,
            }
        };
        match decision {
            Capture::PassThrough => {}
            Capture::Swallow => return LRESULT(1),
            Capture::SwallowAndStop => {
                let thread_id = SESSION.lock().unwrap().as_ref().map_or(0, |s| s.thread_id);
                if thread_id != 0 {
                    let _ = unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
                }
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// 会话活跃时单个按键事件的处理决策。
#[derive(Debug, PartialEq, Eq)]
enum Capture {
    /// 放行（非按键消息、会话已被接管等）。
    PassThrough,
    /// 吞掉，会话继续等待（修饰键、主键抬起）。
    Swallow,
    /// 吞掉并结束会话（截获完成、Esc/未知键、超时）。
    SwallowAndStop,
}

/// 会话活跃时的按键处理：修饰键态由钩子事件维护，修饰键与主键抬起吞掉但
/// 继续等待；非修饰键按下截获为组合键；超时、Esc、无法映射的键分别以
/// Timeout / Cancelled 结束会话。
fn capture_if_recording(session: &mut Session, wparam: WPARAM, lparam: LPARAM) -> Capture {
    let message = wparam.0 as u32;
    let key = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let is_key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let is_key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
    if !is_key_down && !is_key_up {
        return Capture::PassThrough;
    }
    if session.started.elapsed() > TIMEOUT {
        session.active = false;
        session.outcome = Some(RecordOutcome::Timeout);
        return Capture::SwallowAndStop;
    }
    // 修饰键态先于分支更新：吞掉的按键不进系统输入管线，只能在这里跟踪
    session.mods.absorb(key.vkCode, is_key_down);
    if is_modifier(key.vkCode) {
        // 修饰键不成组合：吞掉并继续等待主键
        return Capture::Swallow;
    }
    if !is_key_down {
        return Capture::Swallow;
    }
    let extended = key.flags & LLKHF_EXTENDED != KBDLLHOOKSTRUCT_FLAGS(0);
    let Some(code) = vk_to_code(key.vkCode, extended) else {
        // 无法映射的键（IME 处理中等）：不绑定含义不明的组合，结束会话
        session.active = false;
        session.outcome = Some(RecordOutcome::Cancelled);
        return Capture::SwallowAndStop;
    };
    session.active = false;
    let combo = session.mods.combo_string(code);
    session.outcome = Some(RecordOutcome::Recorded(combo));
    Capture::SwallowAndStop
}

/// 低级钩子的 vkCode 为左右变体键码（VK_LSHIFT 等），泛型码一并兜住。
fn is_modifier(vk: u32) -> bool {
    matches!(
        VIRTUAL_KEY(vk as u16),
        VK_SHIFT
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_CONTROL
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_MENU
            | VK_LMENU
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
    )
}

/// 虚拟键 → KeyboardEvent.code 命名（组合键解析端可识别的名字）。
/// `extended` 区分小键盘变体（如 NumPadEnter）；映射不出的键返回 None。
fn vk_to_code(vk: u32, extended: bool) -> Option<&'static str> {
    // 连续段用数值区间；等值臂用常量模式（VIRTUAL_KEY 派生结构等值）
    if (0x30..=0x39).contains(&vk) {
        return Some(DIGITS[(vk - 0x30) as usize]);
    }
    if (0x41..=0x5A).contains(&vk) {
        return Some(LETTERS[(vk - 0x41) as usize]);
    }
    if (VK_F1.0 as u32..=VK_F24.0 as u32).contains(&vk) {
        return Some(F_KEYS[(vk - VK_F1.0 as u32) as usize]);
    }
    if (VK_NUMPAD0.0 as u32..=VK_NUMPAD9.0 as u32).contains(&vk) {
        return Some(NUMPADS[(vk - VK_NUMPAD0.0 as u32) as usize]);
    }
    Some(match VIRTUAL_KEY(vk as u16) {
        VK_BACK => "Backspace",
        VK_CAPITAL => "CapsLock",
        VK_DELETE => "Delete",
        VK_DOWN => "ArrowDown",
        VK_END => "End",
        VK_HOME => "Home",
        VK_INSERT => "Insert",
        VK_LEFT => "ArrowLeft",
        VK_NEXT => "PageDown",
        VK_NUMLOCK => "NumLock",
        VK_ADD => "NumpadAdd",
        VK_DECIMAL => "NumpadDecimal",
        VK_DIVIDE => "NumpadDivide",
        VK_MULTIPLY => "NumpadMultiply",
        VK_RETURN if extended => "NumpadEnter",
        VK_RETURN => "Enter",
        VK_OEM_1 => "Semicolon",
        VK_OEM_2 => "Slash",
        VK_OEM_3 => "Backquote",
        VK_OEM_4 => "BracketLeft",
        VK_OEM_5 => "Backslash",
        VK_OEM_6 => "BracketRight",
        VK_OEM_7 => "Quote",
        VK_OEM_COMMA => "Comma",
        VK_OEM_MINUS => "Minus",
        VK_OEM_PERIOD => "Period",
        VK_OEM_PLUS => "Equal",
        VK_PRIOR => "PageUp",
        VK_RIGHT => "ArrowRight",
        VK_SNAPSHOT => "PrintScreen",
        VK_SCROLL => "ScrollLock",
        VK_PAUSE => "Pause",
        VK_SPACE => "Space",
        VK_SUBTRACT => "NumpadSubtract",
        VK_TAB => "Tab",
        VK_UP => "ArrowUp",
        _ => return None,
    })
}

const DIGITS: [&str; 10] = [
    "Digit0", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8",
    "Digit9",
];
const LETTERS: [&str; 26] = [
    "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ", "KeyK", "KeyL",
    "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT", "KeyU", "KeyV", "KeyW", "KeyX",
    "KeyY", "KeyZ",
];
const NUMPADS: [&str; 10] = [
    "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7",
    "Numpad8", "Numpad9",
];
const F_KEYS: [&str; 24] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15",
    "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
];

#[cfg(test)]
mod tests {
    use super::*;

    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_F12, VK_NUMPAD5};

    #[test]
    fn letters_digits_space_map_to_code_names() {
        assert_eq!(vk_to_code(0x41, false), Some("KeyA"));
        assert_eq!(vk_to_code(0x5A, false), Some("KeyZ"));
        assert_eq!(vk_to_code(0x30, false), Some("Digit0"));
        assert_eq!(vk_to_code(0x39, false), Some("Digit9"));
        assert_eq!(vk_to_code(VK_SPACE.0 as u32, false), Some("Space"));
    }

    #[test]
    fn enter_distinguishes_numpad_by_extended_flag() {
        assert_eq!(vk_to_code(VK_RETURN.0 as u32, false), Some("Enter"));
        assert_eq!(vk_to_code(VK_RETURN.0 as u32, true), Some("NumpadEnter"));
    }

    #[test]
    fn punctuation_arrows_numpad_map_to_parseable_names() {
        assert_eq!(vk_to_code(VK_OEM_1.0 as u32, false), Some("Semicolon"));
        assert_eq!(vk_to_code(VK_OEM_PLUS.0 as u32, false), Some("Equal"));
        assert_eq!(vk_to_code(VK_OEM_COMMA.0 as u32, false), Some("Comma"));
        assert_eq!(vk_to_code(VK_OEM_4.0 as u32, false), Some("BracketLeft"));
        assert_eq!(vk_to_code(VK_UP.0 as u32, false), Some("ArrowUp"));
        assert_eq!(vk_to_code(VK_PRIOR.0 as u32, false), Some("PageUp"));
        assert_eq!(vk_to_code(VK_NUMPAD5.0 as u32, false), Some("Numpad5"));
        assert_eq!(vk_to_code(VK_ADD.0 as u32, false), Some("NumpadAdd"));
        assert_eq!(vk_to_code(VK_F12.0 as u32, false), Some("F12"));
        assert_eq!(vk_to_code(VK_SNAPSHOT.0 as u32, false), Some("PrintScreen"));
    }

    #[test]
    fn unmapdable_keys_return_none() {
        // VK_PROCESSKEY（IME 处理中）等无组合键语义的键；左右变体键码是
        // 修饰键，永远不该作为主键出现在映射表里
        assert_eq!(vk_to_code(0xE5, false), None);
        assert_eq!(vk_to_code(0x00, false), None);
        assert_eq!(vk_to_code(VK_LSHIFT.0 as u32, false), None);
        assert_eq!(vk_to_code(VK_RMENU.0 as u32, false), None);
    }

    #[test]
    fn left_right_variants_are_recognized_as_modifiers() {
        // 低级钩子的 vkCode 是左右变体：泛型码（VK_SHIFT 等）在钩子里不出现
        assert!(is_modifier(VK_SHIFT.0 as u32));
        assert!(is_modifier(VK_LSHIFT.0 as u32));
        assert!(is_modifier(VK_RSHIFT.0 as u32));
        assert!(is_modifier(VK_LCONTROL.0 as u32));
        assert!(is_modifier(VK_RCONTROL.0 as u32));
        assert!(is_modifier(VK_LMENU.0 as u32));
        assert!(is_modifier(VK_RMENU.0 as u32));
        assert!(is_modifier(VK_LWIN.0 as u32));
        assert!(is_modifier(VK_RWIN.0 as u32));
        assert!(!is_modifier(VK_SPACE.0 as u32));
        assert!(!is_modifier(0x41));
    }

    #[test]
    fn code_names_are_all_parseable_by_hotkey_parser() {
        // 组合键串最终要过 global_hotkey 的解析器：全部映射名必须可解析
        for vk in 0x01u32..=0xFF {
            let Some(code) = vk_to_code(vk, false) else {
                continue;
            };
            let combo = format!("alt+{code}");
            assert!(
                crate::shortcuts::parse_combo(&combo).is_ok(),
                "alt+{code}（vk 0x{vk:02X}）应可解析"
            );
        }
    }

    /// 构造会话 + 钩子事件参数，驱动 capture_if_recording 断言决策语义。
    fn capture(session: &mut Session, message: u32, vk: u32, extended: bool) -> Capture {
        let key = KBDLLHOOKSTRUCT {
            vkCode: vk,
            scanCode: 0,
            flags: if extended {
                LLKHF_EXTENDED
            } else {
                KBDLLHOOKSTRUCT_FLAGS(0)
            },
            time: 0,
            dwExtraInfo: 0,
        };
        capture_if_recording(
            session,
            WPARAM(message as usize),
            LPARAM(&key as *const KBDLLHOOKSTRUCT as isize),
        )
    }

    fn fresh_session() -> Session {
        Session {
            generation: 1,
            active: true,
            started: Instant::now(),
            thread_id: 0,
            mods: Mods::default(),
            outcome: None,
            sink: Arc::new(|_| {}),
        }
    }

    #[test]
    fn modifier_keydown_swallows_and_keeps_waiting() {
        let mut session = fresh_session();
        let decision = capture(&mut session, WM_SYSKEYDOWN, VK_LMENU.0 as u32, false);
        assert_eq!(decision, Capture::Swallow);
        assert!(session.active);
        assert!(session.outcome.is_none());
    }

    #[test]
    fn main_key_keydown_captures_combo_and_stops() {
        let mut session = fresh_session();
        let decision = capture(&mut session, WM_SYSKEYDOWN, VK_SPACE.0 as u32, false);
        assert_eq!(decision, Capture::SwallowAndStop);
        assert!(!session.active);
        assert!(matches!(session.outcome, Some(RecordOutcome::Recorded(_))));
    }

    #[test]
    fn escape_cancels_and_stops() {
        let mut session = fresh_session();
        let decision = capture(&mut session, WM_KEYDOWN, VK_ESCAPE.0 as u32, false);
        assert_eq!(decision, Capture::SwallowAndStop);
        assert!(!session.active);
        assert!(matches!(session.outcome, Some(RecordOutcome::Cancelled)));
    }

    #[test]
    fn unknown_key_cancels_and_stops() {
        let mut session = fresh_session();
        let decision = capture(&mut session, WM_KEYDOWN, 0xE5, false);
        assert_eq!(decision, Capture::SwallowAndStop);
        assert!(matches!(session.outcome, Some(RecordOutcome::Cancelled)));
    }

    #[test]
    fn key_up_of_main_key_swallows_and_keeps_waiting() {
        let mut session = fresh_session();
        let decision = capture(&mut session, WM_KEYUP, VK_SPACE.0 as u32, false);
        assert_eq!(decision, Capture::Swallow);
        assert!(session.active);
    }

    #[test]
    fn non_key_messages_pass_through() {
        let mut session = fresh_session();
        let decision = capture(&mut session, 0x0007, VK_SPACE.0 as u32, false); // WM_MOUSEMOVE
        assert_eq!(decision, Capture::PassThrough);
        assert!(session.active);
    }

    /// 回归：修饰键态必须来自钩子事件而非异步键表——被钩子吞掉的按键不会
    /// 更新 GetAsyncKeyState，读异步键表会把组合键录成裸主键。
    #[test]
    fn modifier_tracked_from_hook_events_not_async_key_state() {
        let mut session = fresh_session();
        // Alt（左变体）按下 → 吞掉继续等待；Space 按下 → 录得 alt+Space
        assert_eq!(
            capture(&mut session, WM_SYSKEYDOWN, VK_LMENU.0 as u32, false),
            Capture::Swallow
        );
        assert_eq!(
            capture(&mut session, WM_SYSKEYDOWN, VK_SPACE.0 as u32, false),
            Capture::SwallowAndStop
        );
        assert_eq!(
            session.outcome,
            Some(RecordOutcome::Recorded("alt+Space".into()))
        );
    }

    #[test]
    fn modifier_keyup_clears_tracked_state() {
        let mut session = fresh_session();
        let _ = capture(&mut session, WM_KEYDOWN, VK_LCONTROL.0 as u32, false);
        let _ = capture(&mut session, WM_KEYUP, VK_LCONTROL.0 as u32, false);
        let _ = capture(&mut session, WM_KEYDOWN, VK_SPACE.0 as u32, false);
        assert_eq!(
            session.outcome,
            Some(RecordOutcome::Recorded("Space".into()))
        );
    }

    #[test]
    fn right_variants_map_to_canonical_modifiers() {
        let mut session = fresh_session();
        let _ = capture(&mut session, WM_SYSKEYDOWN, VK_RMENU.0 as u32, false);
        let _ = capture(&mut session, WM_KEYDOWN, VK_RSHIFT.0 as u32, false);
        let _ = capture(&mut session, WM_SYSKEYDOWN, VK_RWIN.0 as u32, false);
        let _ = capture(&mut session, WM_KEYDOWN, VK_OEM_COMMA.0 as u32, false);
        assert_eq!(
            session.outcome,
            Some(RecordOutcome::Recorded("alt+shift+super+Comma".into()))
        );
    }

    /// 录制开始前已按下的修饰键不产生钩子事件：以开始时刻的异步键表为种子，
    /// 泛型/左右变体键码都归到规范修饰键位。
    #[test]
    fn pre_held_modifier_seeded_from_async_key_state() {
        let seeded = Mods::seeded_with(|vk| vk == VK_MENU || vk == VK_RWIN);
        assert!(seeded.alt);
        assert!(seeded.super_key);
        assert!(!seeded.ctrl);
        assert!(!seeded.shift);
    }

    /// 轮询兜底：钩子被抢位时按键流入异步键表，按下沿按「跟踪态 ∪ 异步态」
    /// 截获组合键；录制前已按住的键不报沿。
    #[test]
    fn poll_edge_captures_combo_with_async_modifiers() {
        let mut watch = EdgeWatch::default();
        let tracked = Mods::default();
        let down = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new()));
        let read = |vk: u32| down.borrow().contains(&vk);
        // 首轮建基线；仅有修饰键按下不成沿，继续等待（异步键表对泛型 VK
        // 查询会反映左右变体按下态，与真实键表一致）
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        down.borrow_mut().insert(VK_LMENU.0 as u32);
        down.borrow_mut().insert(VK_MENU.0 as u32);
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        // 主键按下沿：修饰键态从异步键表并入
        down.borrow_mut().insert(VK_SPACE.0 as u32);
        let (vk, mods) = scan_press_edges(&mut watch, &tracked, read).expect("应报按下沿");
        assert_eq!(vk, VK_SPACE.0 as u32);
        assert_eq!(
            poll_outcome(vk, &mods),
            RecordOutcome::Recorded("alt+Space".into())
        );
    }

    #[test]
    fn poll_baseline_ignores_pre_held_keys_until_repress() {
        let mut watch = EdgeWatch::default();
        let tracked = Mods::default();
        let down = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new()));
        down.borrow_mut().insert(VK_SPACE.0 as u32);
        let read = |vk: u32| down.borrow().contains(&vk);
        // 建基线时已按住 → 不算新按下；持续按住也不报
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        down.borrow_mut().remove(&(VK_SPACE.0 as u32));
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        down.borrow_mut().insert(VK_SPACE.0 as u32);
        assert!(scan_press_edges(&mut watch, &tracked, read).is_some());
    }

    /// 钩子吞掉的修饰键不进异步键表：轮询并入钩子跟踪态，不丢前缀。
    #[test]
    fn poll_merges_hook_tracked_modifiers() {
        let mut watch = EdgeWatch::default();
        let tracked = Mods {
            alt: true,
            ..Mods::default()
        };
        let down = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new()));
        let read = |vk: u32| down.borrow().contains(&vk);
        assert_eq!(scan_press_edges(&mut watch, &tracked, read), None);
        down.borrow_mut().insert(VK_SPACE.0 as u32);
        let (vk, mods) = scan_press_edges(&mut watch, &tracked, read).expect("应报按下沿");
        assert_eq!(
            poll_outcome(vk, &mods),
            RecordOutcome::Recorded("alt+Space".into())
        );
    }

    #[test]
    fn poll_esc_and_unmapped_key_end_session_cancelled() {
        let mut watch = EdgeWatch::default();
        let tracked = Mods::default();
        let down = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new()));
        let read = |vk: u32| down.borrow().contains(&vk);
        let _ = scan_press_edges(&mut watch, &tracked, read);
        down.borrow_mut().insert(VK_ESCAPE.0 as u32);
        let (vk, mods) = scan_press_edges(&mut watch, &tracked, read).expect("应报按下沿");
        assert_eq!(poll_outcome(vk, &mods), RecordOutcome::Cancelled);

        let mut watch = EdgeWatch::default();
        let _ = scan_press_edges(&mut watch, &tracked, read);
        down.borrow_mut().insert(0xE5); // VK_PROCESSKEY（IME 处理中）
        let (vk, mods) = scan_press_edges(&mut watch, &tracked, read).expect("应报按下沿");
        assert_eq!(poll_outcome(vk, &mods), RecordOutcome::Cancelled);
    }
}
