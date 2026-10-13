//! Win32 后端：Ctrl+C 键击模拟（ADR-0001 划词取词 seam）。
//!
//! 平台实现不做单测：send_copy 会向用户前台注入键击，靠手动验收。

use windows::Win32::UI::Input::KeyboardAndMouse::VK_C;

use crate::platform::{PlatformError, SelectionAccess};

use super::{foreground_hwnd, process_of, send_ctrl_chord};

pub struct WinSelectionAccess;

impl SelectionAccess for WinSelectionAccess {
    fn foreground_process(&self) -> Option<u32> {
        foreground_hwnd().and_then(process_of)
    }

    fn send_copy(&self) -> Result<(), PlatformError> {
        send_ctrl_chord(VK_C)
    }
}
