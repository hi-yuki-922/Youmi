//! Windows 平台实现（唯一实现的后端，见 ADR-0001）。

pub mod apps;
pub mod bmp;
pub mod clipboard;
pub mod pasteback;
pub mod secrets;
pub mod selection;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::platform::PlatformError;

/// 当前前台窗口句柄；取不到时返回 None。
pub(crate) fn foreground_hwnd() -> Option<isize> {
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.0.is_null()).then_some(hwnd.0 as isize)
}

/// 窗口所属进程 ID；窗口已失效（解析不出进程）返回 None。
pub(crate) fn process_of(hwnd: isize) -> Option<u32> {
    if hwnd == 0 {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

/// 向前台窗口注入一次 Ctrl+vk 键击：Ctrl 按下 → vk 按下 → vk 抬起 → Ctrl 抬起；
/// vkey 方式，接收组合键的常规应用均可识别。
pub(crate) fn send_ctrl_chord(vk: VIRTUAL_KEY) -> Result<(), PlatformError> {
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
        key(vk, false),
        key(vk, true),
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
