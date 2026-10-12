//! Win32 后端：前台窗口管理与 Ctrl+V 键击模拟（ADR-0001 回帖 seam）。
//!
//! 平台实现不做单测：行为由 `#[ignore]` smoke 测试 + 手动验收覆盖。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

use crate::platform::{PastebackAccess, PlatformError};

pub struct WinPastebackAccess;

impl PastebackAccess for WinPastebackAccess {
    fn foreground_window(&self) -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        (!hwnd.0.is_null()).then_some(hwnd.0 as isize)
    }

    fn window_process(&self, hwnd: isize) -> Option<u32> {
        if hwnd == 0 {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid)) };
        (pid != 0).then_some(pid)
    }

    fn set_foreground(&self, hwnd: isize) -> bool {
        unsafe { SetForegroundWindow(HWND(hwnd as *mut _)) }.as_bool()
    }

    fn send_paste(&self) -> Result<(), PlatformError> {
        // Ctrl 按下 → V 按下 → V 抬起 → Ctrl 抬起；vkey 方式，接收 Ctrl+V 的常规应用均可识别
        let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                },
            },
        };
        let inputs = [
            key(VK_CONTROL, false),
            key(VK_V, false),
            key(VK_V, true),
            key(VK_CONTROL, true),
        ];
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            return Err(PlatformError(format!(
                "SendInput 只注入了 {sent}/{} 个键击事件",
                inputs.len()
            )));
        }
        Ok(())
    }
}

/// 真实前台窗口 smoke：跑在实机上只读，默认忽略，用 `cargo test -- --ignored` 手动执行。
/// send_paste 会向用户前台窗口注入键击，不设自动 smoke，靠手动验收。
#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    #[ignore = "读取真实前台窗口，手动执行：cargo test -- --ignored"]
    fn foreground_window_is_readable() {
        let hwnd = WinPastebackAccess
            .foreground_window()
            .expect("桌面会话中应有前台窗口");
        let pid = WinPastebackAccess
            .window_process(hwnd)
            .expect("前台窗口应可解析出进程 ID");
        assert_ne!(pid, 0);
    }
}
