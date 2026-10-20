//! 剪贴板历史的 tauri command 边界：前端只经这里访问能力。

use std::sync::Arc;

use base64::Engine as _;
use tauri::{Emitter as _, Manager, State};

use super::engine::ClipboardEngine;
use super::model::ClipboardKind;
use super::pasteback::{self, PastebackOutcome, PastebackState};
use super::repository::{ClipboardEntry, ClipboardHistoryRepository, HistoryQuery};
use crate::platform::{ClipboardAccess as _, WinClipboardAccess};

/// 默认单次查询条数。
const DEFAULT_QUERY_LIMIT: u32 = 100;

/// 面板回填回调：发送插入事件并聚焦面板，返回事件是否发送成功。
type PaletteTextInserter = Box<dyn Fn(&str) -> bool + Send>;

pub struct ClipboardState {
    pub engine: Arc<ClipboardEngine>,
    pub repo: Arc<ClipboardHistoryRepository>,
}

/// 引擎是否已初始化（数据库损坏等会走降级路径，State 不被管理）；
/// false 时历史与容量设置均不生效，前端据此呈现失效态。
#[tauri::command]
pub fn clipboard_engine_available(app: tauri::AppHandle) -> bool {
    app.try_state::<ClipboardState>().is_some()
}

#[tauri::command]
pub fn clipboard_query(
    state: State<'_, ClipboardState>,
    keyword: Option<String>,
    kind: Option<ClipboardKind>,
    limit: Option<u32>,
) -> Result<Vec<ClipboardEntry>, String> {
    let query = HistoryQuery {
        keyword,
        kind,
        limit: limit.unwrap_or(DEFAULT_QUERY_LIMIT).clamp(1, 10_000),
    };
    state.repo.query(&query).map_err(|e| e.to_string())
}

/// 取条目的 PNG 字节，base64 编码返回；文本条目返回 None。
#[tauri::command]
pub fn clipboard_get_image(
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<Option<String>, String> {
    state
        .repo
        .image_data(id)
        .map(|png| png.map(|data| base64::engine::general_purpose::STANDARD.encode(data)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clipboard_delete(state: State<'_, ClipboardState>, id: i64) -> Result<bool, String> {
    state.repo.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clipboard_clear(state: State<'_, ClipboardState>) -> Result<usize, String> {
    state.repo.clear().map_err(|e| e.to_string())
}

/// 剪贴板历史容量（条数），超出淘汰最旧，缩容立即生效。
#[tauri::command]
pub fn clipboard_set_capacity(
    state: State<'_, ClipboardState>,
    capacity: u32,
) -> Result<(), String> {
    state
        .repo
        .set_capacity(capacity.clamp(1, 1_000_000) as usize)
        .map_err(|e| e.to_string())
}

/// 用户侧复制通道（工具页复制按钮等）：写文本，监听器照常捕获入史。
/// 与下方自家通道构成仅有的两条前端写路径。不依赖引擎状态——引擎降级时
/// 复制仍可用，只是不入史。
#[tauri::command]
pub fn clipboard_write_text(text: String) -> Result<(), String> {
    WinClipboardAccess.write_text(&text).map_err(|e| e.0)
}

/// 自家写入通道（密码复制、划词模拟复制等）：写剪贴板但不进入历史。
#[tauri::command]
pub fn clipboard_write_own_text(
    state: State<'_, ClipboardState>,
    text: String,
) -> Result<(), String> {
    state.engine.write_own_text(&text).map_err(|e| e.0)
}

#[tauri::command]
pub fn clipboard_write_own_image_png(
    state: State<'_, ClipboardState>,
    base64_png: String,
) -> Result<(), String> {
    let png = base64::engine::general_purpose::STANDARD
        .decode(base64_png)
        .map_err(|e| format!("PNG base64 解码失败: {e}"))?;
    state.engine.write_own_image_png(&png).map_err(|e| e.0)
}

/// 回帖：条目写剪贴板（自家通道不入史）→ 浮层隐藏、焦点交还呼出浮层前的
/// 窗口 → 模拟 Ctrl+V → 延迟恢复用户原剪贴板。无有效目标或焦点/键击失败
/// 时降级为 `copied-only`（条目留在剪贴板供手动粘贴）。
/// 命令面板可见时例外：无有效目标的文本条目回填面板输入框（`inserted-to-palette`）。
#[tauri::command]
pub async fn clipboard_pasteback(
    app: tauri::AppHandle,
    state: State<'_, ClipboardState>,
    pasteback: State<'_, PastebackState>,
    id: i64,
) -> Result<PastebackOutcome, String> {
    // 面板可见 = 浮层共存上下文（呼出剪贴板浮层时前台是面板，捕获目标为空）：
    // 文本条目回填面板输入框而非降级仅复制
    let palette_open = app
        .get_webview_window(crate::palette::LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(false));
    let insert_palette_text: Option<PaletteTextInserter> = palette_open.then({
        let app = app.clone();
        move || {
            Box::new(move |text: &str| -> bool {
                if let Err(e) = app.emit_to(
                    crate::palette::LABEL,
                    crate::palette::INSERT_TEXT_EVENT,
                    text,
                ) {
                    eprintln!("youmi: 面板回填事件发送失败: {e}");
                    return false;
                }
                if let Some(palette) = app.get_webview_window(crate::palette::LABEL) {
                    let _ = palette.set_focus();
                }
                true
            }) as Box<dyn Fn(&str) -> bool + Send>
        }
    });
    let deps = pasteback::PastebackDeps {
        repo: Arc::clone(&state.repo),
        engine: Arc::clone(&state.engine),
        input: Arc::clone(&pasteback.platform),
        target: *pasteback.target.lock().unwrap(),
        main_window: pasteback::main_window_hwnd(&app),
        own_pid: std::process::id(),
        focus_settle: pasteback::FOCUS_SETTLE,
        restore_delay: pasteback::RESTORE_DELAY,
    };
    // 编排含焦点等待与键击注入，放阻塞线程池，不占 async runtime
    tauri::async_runtime::spawn_blocking(move || {
        let insert = insert_palette_text
            .as_deref()
            .map(|f| f as &dyn Fn(&str) -> bool);
        pasteback::run(
            &deps,
            id,
            &|| {
                if let Some(window) = app.get_webview_window(crate::CLIPBOARD_OVERLAY_LABEL) {
                    let _ = window.hide();
                }
            },
            insert,
        )
    })
    .await
    .map_err(|e| format!("回帖任务执行失败: {e}"))?
}
