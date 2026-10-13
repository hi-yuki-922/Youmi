//! 划词翻译：全局快捷键取词 → 翻译浮层流式呈现。本模块是 tauri command
//! 与快捷键入口的边界；取词编排见 `selection`，DeepSeek API key 存系统
//! 凭据库（凭据存储 seam），模型名存设置文件（非秘密）。

pub mod selection;

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter as _, Manager as _, State};

use crate::clipboard::engine::ClipboardEngine;
use crate::platform::{SecretStore, SelectionAccess};

/// 翻译浮层窗口 label。
pub const LABEL: &str = "translate-overlay";
/// 取词结果事件：发往翻译浮层，payload 见 [`SelectionEventPayload`]。
pub const SELECTION_EVENT: &str = "translate-selection";
/// DeepSeek API key 在凭据库中的条目名。
const DEEPSEEK_API_KEY: &str = "deepseek-api-key";

/// 取词结果事件载荷：`text` = 取到的选中文本；`message` = 取词失败/无选中
/// 的说明。互斥由类型表达，不发双空/双值载荷。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SelectionEventPayload {
    Text { text: String },
    Message { message: String },
}

/// 取词依赖：剪贴板引擎（自家写入过滤与恢复）+ 划词取词平台 seam。
pub struct TranslateState {
    pub engine: Arc<ClipboardEngine>,
    pub selection: Arc<dyn SelectionAccess>,
}

/// 秘密存取依赖。独立于取词引擎管理：引擎初始化失败时设置页仍可配置。
pub struct SecretState {
    pub store: Arc<dyn SecretStore>,
}

/// Alt+T 入口：取词 → 浮层移到光标旁显示聚焦 → 取词结果发事件给浮层。
/// 取词必须先于显示（Ctrl+C 要发给呼起时仍在前台的目标应用），编排含键击
/// 注入与轮询等待，由调用方放阻塞线程池执行。
pub fn capture_and_show(app: &AppHandle) {
    let Some(state) = app.try_state::<TranslateState>() else {
        // 引擎未初始化的降级：浮层照常呼出并说明原因
        show_with_payload(
            app,
            &SelectionEventPayload::Message {
                message: "剪贴板引擎未初始化，划词翻译不可用".into(),
            },
        );
        return;
    };

    // 自家窗口（主窗口/浮层）聚焦时静默跳过：Ctrl+C 不落进 Youmi 自己的
    // 界面，也不把已显示的译文清成错误提示（与回帖"自家窗口不作目标"一致）
    if state
        .selection
        .foreground_process()
        .is_none_or(|pid| pid == std::process::id())
    {
        return;
    }

    let deps = selection::SelectionDeps {
        engine: Arc::clone(&state.engine),
        input: Arc::clone(&state.selection),
        own_pid: std::process::id(),
        settle: selection::SETTLE,
        timeout: selection::TIMEOUT,
        poll_interval: selection::POLL_INTERVAL,
    };
    let payload = match selection::capture(&deps) {
        Ok(selection::Selection::Text(text)) => SelectionEventPayload::Text { text },
        Ok(selection::Selection::Nothing) => SelectionEventPayload::Message {
            message: "未检测到选中文本".into(),
        },
        Err(message) => SelectionEventPayload::Message {
            message: format!("取词失败：{message}"),
        },
    };
    show_with_payload(app, &payload);
}

fn show_with_payload(app: &AppHandle, payload: &SelectionEventPayload) {
    if let Err(e) = crate::overlay::show_at_cursor(app, LABEL) {
        eprintln!("youmi: 呼出划词翻译浮层失败: {e}");
        return;
    }
    if let Err(e) = app.emit_to(LABEL, SELECTION_EVENT, payload) {
        eprintln!("youmi: 发送取词结果事件失败: {e}");
    }
}

/// 凭据库中是否已配置 API key。设置页展示"已配置"状态用：presence 查询
/// 不把 key 明文带进 webview，取回明文仅翻译发起时（translate_get_api_key）。
#[tauri::command]
pub fn translate_has_api_key(state: State<'_, SecretState>) -> Result<bool, String> {
    state
        .store
        .get(DEEPSEEK_API_KEY)
        .map(|value| value.is_some())
        .map_err(|e| e.0)
}

#[tauri::command]
pub fn translate_get_api_key(state: State<'_, SecretState>) -> Result<Option<String>, String> {
    state.store.get(DEEPSEEK_API_KEY).map_err(|e| e.0)
}

/// 保存 API key；空白 = 清除（凭据库中删除），保存值去首尾空白。
fn save_api_key(store: &dyn SecretStore, key: &str) -> Result<(), crate::platform::PlatformError> {
    let key = key.trim();
    if key.is_empty() {
        store.delete(DEEPSEEK_API_KEY)
    } else {
        store.set(DEEPSEEK_API_KEY, key)
    }
}

#[tauri::command]
pub fn translate_set_api_key(state: State<'_, SecretState>, key: String) -> Result<(), String> {
    save_api_key(state.store.as_ref(), &key).map_err(|e| e.0)
}

#[cfg(test)]
mod smoke_tests {
    use super::DEEPSEEK_API_KEY;
    use crate::platform::{SecretStore as _, WinSecretStore};

    /// 真实凭据库 smoke：验证 `deepseek-api-key` 条目经平台 seam 可读（命名
    /// 与 keyring windows-native 一致）。条目不存在时仅打印状态，不算失败。
    #[test]
    #[ignore = "读真实系统凭据库，手动执行：cargo test -- --ignored --test-threads=1"]
    fn deepseek_api_key_entry_is_readable() {
        let value = WinSecretStore.get(DEEPSEEK_API_KEY).unwrap();
        println!("deepseek-api-key configured: {}", value.is_some());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// 假凭据库：内存键值表，语义与真实凭据库一致（delete 不存在视为成功）。
    struct FakeStore(Mutex<std::collections::HashMap<String, String>>);

    impl FakeStore {
        fn new() -> Arc<Self> {
            Arc::new(Self(Mutex::new(std::collections::HashMap::new())))
        }
    }

    impl SecretStore for FakeStore {
        fn get(&self, key: &str) -> Result<Option<String>, crate::platform::PlatformError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }

        fn set(&self, key: &str, value: &str) -> Result<(), crate::platform::PlatformError> {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn delete(&self, key: &str) -> Result<(), crate::platform::PlatformError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn empty_key_save_deletes_credential() {
        let store = FakeStore::new();
        store.set(DEEPSEEK_API_KEY, "sk-existing").unwrap();

        save_api_key(store.as_ref(), "   ").unwrap();

        assert_eq!(store.get(DEEPSEEK_API_KEY).unwrap(), None);
    }

    #[test]
    fn api_key_is_trimmed_on_save() {
        let store = FakeStore::new();

        save_api_key(store.as_ref(), "  sk-abc  ").unwrap();

        assert_eq!(store.get(DEEPSEEK_API_KEY).unwrap(), Some("sk-abc".into()));
    }
}
