//! 剪贴板历史的 tauri command 边界：前端只经这里访问能力。

use std::sync::Arc;

use base64::Engine as _;
use tauri::State;

use super::engine::ClipboardEngine;
use super::model::ClipboardKind;
use super::repository::{ClipboardEntry, ClipboardHistoryRepository, HistoryQuery};

/// 默认单次查询条数。
const DEFAULT_QUERY_LIMIT: u32 = 100;

pub struct ClipboardState {
    pub engine: Arc<ClipboardEngine>,
    pub repo: Arc<ClipboardHistoryRepository>,
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
