//! Windows 平台实现（唯一实现的后端，见 ADR-0001）。

pub mod apps;
pub mod bmp;
pub mod clipboard;
pub mod pasteback;
pub mod record;
pub mod secrets;
pub mod selection;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
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

/// 构造一个 vkey 按下/抬起键击事件。
fn key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
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
    }
}

/// 键当前是否处于按下态：物理按键与注入键击都会更新异步键表。
fn key_is_down(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000) != 0
}

/// 向前台窗口注入一次 Ctrl+vk 键击：Ctrl 按下 → vk 按下 → vk 抬起 → Ctrl 抬起；
/// vkey 方式，接收组合键的常规应用均可识别。
///
/// 全局快捷键（如 Alt+T）触发瞬间用户往往仍按着修饰键，不清掉的话目标应用
/// 收到的是 Ctrl+Alt+C 这类被污染的组合而非 Ctrl+vk，复制永不发生——chord
/// 前先抬起残留的 Alt/Shift/Win。Ctrl 是 chord 自身的组成部分，残留无害，
/// 不处理；抬起后不回按，回按会让目标应用看到多余的 Alt 按下（菜单激活等副作用）。
pub(crate) fn send_ctrl_chord(vk: VIRTUAL_KEY) -> Result<(), PlatformError> {
    let mut inputs: Vec<INPUT> = [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
        .into_iter()
        .filter(|lingering| key_is_down(*lingering))
        .map(|lingering| key_input(lingering, true))
        .collect();
    inputs.extend([
        key_input(VK_CONTROL, false),
        key_input(vk, false),
        key_input(vk, true),
        key_input(VK_CONTROL, true),
    ]);
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(PlatformError(format!(
            "SendInput 只注入了 {sent}/{} 个键击事件",
            inputs.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod smoke_tests {
    use std::time::Duration;

    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_C, VK_MENU};

    use super::*;

    /// 真实键击 smoke：全局快捷键（Alt+T）触发瞬间用户仍按着 Alt，chord 注入
    /// 前必须先把残留修饰键抬起，否则目标应用收到 Ctrl+Alt+C 而非 Ctrl+C。
    /// 注入 Alt 按下模拟该状态，断言 chord 后 Alt 已释放。会向真实前台应用
    /// 注入一次 Ctrl+C（复制选区，无害），手动执行。
    #[test]
    #[ignore = "向真实桌面注入键击，手动执行：cargo test -- --ignored --test-threads=1"]
    fn send_ctrl_chord_releases_lingering_alt() {
        let send = |inputs: &[INPUT]| {
            assert_eq!(
                unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) },
                inputs.len() as u32,
                "环境不支持键击注入，smoke 前置不成立"
            );
        };

        // 用户仍按着 Alt（快捷键触发瞬间的真实状态）
        send(&[key_input(VK_MENU, false)]);
        std::thread::sleep(Duration::from_millis(50));
        assert!(key_is_down(VK_MENU), "前置失败：Alt 按下态未生效");

        send_ctrl_chord(VK_C).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let lingering = key_is_down(VK_MENU);
        // 先清理再断言：失败也不把残留的 Alt 态留给用户会话
        send(&[key_input(VK_MENU, true)]);

        assert!(
            !lingering,
            "chord 后 Alt 仍为按下态：目标应用收到 Ctrl+Alt+C 而非 Ctrl+C"
        );
    }
}
