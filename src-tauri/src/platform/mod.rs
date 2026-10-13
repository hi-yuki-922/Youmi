//! ADR-0001 平台 seam：OS 耦合能力收敛在本模块，业务代码只依赖这里的 trait。
//!
//! 仅实现 Windows 后端（`windows` 子模块）；未来移植 = 逐 seam 补第二实现，
//! 不为跨平台做任何前期投资。每个 seam 只有一个实现看起来像过度设计，是有意为之。

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::apps::launch_lnk;
#[cfg(windows)]
pub use windows::clipboard::{WinClipboardAccess, WinClipboardWatcher};
#[cfg(windows)]
pub use windows::pasteback::WinPastebackAccess;
#[cfg(windows)]
pub use windows::secrets::WinSecretStore;
#[cfg(windows)]
pub use windows::selection::WinSelectionAccess;

use std::fmt;

use crate::clipboard::model::ClipboardContent;

/// 平台操作失败。
#[derive(Debug)]
pub struct PlatformError(pub String);

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlatformError {}

/// 剪贴板读写 seam（ADR-0001 clipboard-listener seam 的读写半边）。
pub trait ClipboardAccess: Send + Sync {
    /// 读取当前剪贴板内容；非文本/图片内容（文件列表、HTML 等）或读取失败返回 None。
    fn read(&self) -> Option<ClipboardContent>;

    /// 剪贴板序号：每次剪贴板内容变化递增且永不复用（Windows 的
    /// GetClipboardSequenceNumber）。自家写入过滤靠它精确识别；不可用时返回 None。
    fn sequence(&self) -> Option<u64>;

    /// 写入文本（替换剪贴板全部内容）。
    fn write_text(&self, text: &str) -> Result<(), PlatformError>;

    /// 写入 PNG 图片（替换剪贴板全部内容）。
    fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError>;
}

/// 剪贴板变化监听 seam。
pub trait ClipboardWatcher: Send {
    /// 阻塞当前线程监听剪贴板变化，每次变化调用一次 `on_change`。
    /// 必须在专用线程上调用（实现内部依赖线程亲和的消息循环）；返回即监听结束。
    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Result<(), PlatformError>;
}

/// 回帖 seam（ADR-0001 模拟输入半边）：前台窗口管理与粘贴键击模拟，
/// 供剪贴板回帖把条目送回呼出浮层前的窗口。
pub trait PastebackAccess: Send + Sync {
    /// 当前前台窗口句柄；取不到时返回 None。
    fn foreground_window(&self) -> Option<isize>;

    /// 窗口所属进程 ID；窗口已失效返回 None。
    fn window_process(&self, hwnd: isize) -> Option<u32>;

    /// 把窗口带到前台；返回是否成功。
    fn set_foreground(&self, hwnd: isize) -> bool;

    /// 向当前前台窗口模拟 Ctrl+V 粘贴键击。
    fn send_paste(&self) -> Result<(), PlatformError>;
}

/// 划词取词 seam（ADR-0001）：向前台窗口模拟 Ctrl+C，配合剪贴板读取
/// 完成取词；剪贴板保存/恢复与历史过滤在业务编排层（不在此 seam 内）。
pub trait SelectionAccess: Send + Sync {
    /// 当前前台窗口所属进程 ID；取不到时返回 None。
    fn foreground_process(&self) -> Option<u32>;

    /// 向当前前台窗口模拟 Ctrl+C 复制键击。
    fn send_copy(&self) -> Result<(), PlatformError>;
}

/// 凭据存储 seam（ADR-0001）：系统凭据库（Windows 凭据管理器）的键值存取，
/// 供 API key 等秘密不落明文文件。
pub trait SecretStore: Send + Sync {
    /// 读取秘密；不存在返回 Ok(None)，读取失败返回 Err。
    fn get(&self, key: &str) -> Result<Option<String>, PlatformError>;

    /// 写入秘密（覆盖已有值）。
    fn set(&self, key: &str, value: &str) -> Result<(), PlatformError>;

    /// 删除秘密；不存在视为已删除。
    fn delete(&self, key: &str) -> Result<(), PlatformError>;
}
