//! 命令面板：面板内选中工具后，主窗口显到前台并切到对应工具页，面板随即收起。

use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::MAIN_WINDOW_LABEL;

pub const LABEL: &str = "palette-overlay";

/// 选中工具后发往主窗口的导航事件，payload 为工具 id（前端 `lib/tools.ts` 注册表的 id）。
pub const OPEN_TOOL_EVENT: &str = "open-tool";

/// 打开工具：主窗口显示聚焦 + 导航事件 + 收起面板。三步均尽力而为，失败不打断。
#[tauri::command]
pub fn palette_open_tool(app: AppHandle, tool_id: String) {
    crate::show_main_window(&app);
    let _ = app.emit_to(MAIN_WINDOW_LABEL, OPEN_TOOL_EVENT, tool_id);
    if let Some(palette) = app.get_webview_window(LABEL) {
        let _ = palette.hide();
    }
}
