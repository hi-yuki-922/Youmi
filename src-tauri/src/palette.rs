//! 命令面板：面板内选中工具后，主窗口显到前台并切到对应工具页，面板随即收起。

use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::MAIN_WINDOW_LABEL;

pub const LABEL: &str = "palette-overlay";

/// 选中工具后发往主窗口的导航事件，payload 为 `{ toolId, text? }`
/// （toolId 对应前端 `lib/tools.ts` 注册表；text 为斜杠命令携带的参数）。
pub const OPEN_TOOL_EVENT: &str = "open-tool";

/// 面板呼起事件（无 payload）：前端据此清空上次查询——浮层共存后焦点从兄弟
/// 浮层回落同样触发 focus，清空不能挂在 focus 上。
pub const RESET_EVENT: &str = "palette-reset";

/// 剪贴板条目回填事件，payload 为条目文本：面板把文本插入输入框光标处。
pub const INSERT_TEXT_EVENT: &str = "palette-insert-text";

/// 打开工具：主窗口显示聚焦 + 导航事件 + 收起面板。三步均尽力而为，失败不打断。
/// `text` 为斜杠命令携带的参数（如 /t 的待译文本），经事件转交目标工具页。
#[tauri::command]
pub fn palette_open_tool(app: AppHandle, tool_id: String, text: Option<String>) {
    crate::show_main_window(&app);
    let payload = serde_json::json!({ "toolId": tool_id, "text": text });
    let _ = app.emit_to(MAIN_WINDOW_LABEL, OPEN_TOOL_EVENT, payload);
    if let Some(palette) = app.get_webview_window(LABEL) {
        let _ = palette.hide();
    }
}
