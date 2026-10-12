//! 开始菜单快捷方式启动：ShellExecuteW 以 shell 语义打开 .lnk。

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::platform::PlatformError;

/// 启动 .lnk：目标、参数、工作目录由快捷方式自身解析。
/// ShellExecute 依赖 COM：先在当前线程初始化 STA，成功的初始化（含 S_FALSE 的
/// 已初始化）用 CoUninitialize 配平；模式冲突（RPC_E_CHANGED_MODE）失败无需释放。
/// 返回值 ≤ 32 按 Win32 约定为失败（快捷方式失效、目标拒启等）。
pub fn launch_lnk(path: &str) -> Result<(), PlatformError> {
    let wide = HSTRING::from(path);
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let hinst = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if com.is_ok() {
        unsafe { CoUninitialize() };
    }
    let code = hinst.0 as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(PlatformError(format!(
            "ShellExecuteW 启动失败，返回值 {code}"
        )))
    }
}
