//! Win32 后端：前台窗口管理与 Ctrl+V 键击模拟（ADR-0001 回帖 seam）。
//!
//! 平台实现不做单测：行为由 `#[ignore]` smoke 测试 + 手动验收覆盖。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_V;
use windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
};

use crate::platform::{PastebackAccess, PlatformError};

use super::{foreground_hwnd, process_of, send_ctrl_chord};

pub struct WinPastebackAccess;

impl PastebackAccess for WinPastebackAccess {
    fn foreground_window(&self) -> Option<isize> {
        foreground_hwnd()
    }

    fn window_process(&self, hwnd: isize) -> Option<u32> {
        process_of(hwnd)
    }

    fn set_foreground(&self, hwnd: isize) -> bool {
        unsafe { SetForegroundWindow(HWND(hwnd as *mut _)) }.as_bool()
    }

    fn awaken_window(&self, hwnd: isize) -> bool {
        let hwnd = HWND(hwnd as *mut _);
        unsafe {
            // 最小化时先还原（SW_SHOW 对最小化窗口只是解除隐藏，不还原尺寸）；
            // 隐藏到托盘的主窗口用 SW_SHOW 显出。ShowWindow 结果不判定：
            // 成败由紧随的 SetForegroundWindow 返回值守门
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd).as_bool()
        }
    }
    fn send_paste(&self) -> Result<(), PlatformError> {
        send_ctrl_chord(VK_V)
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
