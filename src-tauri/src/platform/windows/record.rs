//! Win32 后端：组合键录制（ADR-0001 快捷键录制 seam）。
//!
//! 录制期经 WH_KEYBOARD_LL 低级钩子截获按键并整体吞掉：Alt+Space（窗口系统
//! 菜单）、Win+E（Shell）这类 OS 保留组合到不了 webview 的 keydown，DOM 层
//! 捕获对它们天然失效。会话为一次性：截获第一个非修饰键组合、Esc 取消、
//! 超时自动取消。端到端行为需真实键击（不做单测），VK 映射与代数状态机为
//! 纯函数单测覆盖。

use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_ADD, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DECIMAL, VK_DELETE, VK_DIVIDE,
    VK_DOWN, VK_END, VK_F1, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT,
    VK_LWIN, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK, VK_NUMPAD0, VK_NUMPAD9, VK_OEM_1, VK_OEM_2,
    VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD,
    VK_OEM_PLUS, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT,
    VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB, VK_UP,
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

/// 录制会话：录制命令、钩子回调、看门狗与钩子线程共享。
/// outcome 由钩子/看门狗/取消写入；钩子线程退出消息循环后统一回调 sink，
/// emit 不发生在钩子回调内（低级钩子回调必须快，超时会被系统摘除钩子）。
struct Session {
    generation: u64,
    active: bool,
    started: Instant,
    /// 钩子线程 id；0 = 线程尚未就绪（安装钩子前）。
    thread_id: u32,
    outcome: Option<RecordOutcome>,
    sink: Arc<dyn Fn(RecordOutcome) + Send + Sync>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

// 本钩子安装所属的会话代数（钩子回调运行在安装线程上）。
thread_local! {
    static HOOK_GENERATION: Cell<u64> = const { Cell::new(0) };
}

pub struct WinShortcutRecorder;

impl ShortcutRecorder for WinShortcutRecorder {
    fn begin(&self, sink: Arc<dyn Fn(RecordOutcome) + Send + Sync>) -> Result<(), String> {
        let mut session = SESSION.lock().unwrap();
        let generation = session.as_ref().map_or(1, |s| s.generation + 1);
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
            .name("shortcut-record-watchdog".into())
            .spawn(move || {
                std::thread::sleep(TIMEOUT);
                stop_session(generation);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn cancel(&self) {
        stop_session(0);
    }
}

/// 结束会话（`generation` 为 0 时不限代数）：置为已取消并通知钩子线程收尾。
fn stop_session(generation: u64) {
    let thread_id = inactivate(generation);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// 结束同代活跃会话（`generation` 为 0 时不限代数）：置为已取消。
/// 返回会话的钩子线程 id（0 = 未就绪）。
fn inactivate(generation: u64) -> u32 {
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
    s.outcome = Some(RecordOutcome::Cancelled);
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

/// 会话活跃时的按键处理：修饰键与主键抬起吞掉但继续等待；非修饰键按下
/// 截获为组合键；超时、Esc、无法映射的键以 Cancelled 结束会话。
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
        session.outcome = Some(RecordOutcome::Cancelled);
        return Capture::SwallowAndStop;
    }
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
    session.outcome = Some(RecordOutcome::Recorded(combo_string(code)));
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

/// 异步键表 → 规范组合键：修饰键序 ctrl, alt, shift, super，主键为
/// KeyboardEvent.code 命名，与动作绑定的格式一致。
fn combo_string(code: &str) -> String {
    let mut combo = String::new();
    if key_is_down(VK_CONTROL) {
        combo.push_str("ctrl+");
    }
    if key_is_down(VK_MENU) {
        combo.push_str("alt+");
    }
    if key_is_down(VK_SHIFT) {
        combo.push_str("shift+");
    }
    if key_is_down(VK_LWIN) || key_is_down(VK_RWIN) {
        combo.push_str("super+");
    }
    combo.push_str(code);
    combo
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
}
