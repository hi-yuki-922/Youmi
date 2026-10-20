//! 全局快捷键：动作与组合键的绑定管理。三个全局工具各有一个可改绑动作，
//! 组合键经设置页修改：Youmi 内部互斥校验先行，注册成功才改状态并落盘；
//! 系统级占用只能靠注册失败感知，失败时回滚旧注册并如实呈现注册状态。
//! 组合键以插件解析格式（如 "alt+v"、"super+alt+space"）存设置文件，与前端共享。

use std::collections::HashMap;
use std::str::FromStr as _;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _, Manager as _};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt as _, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt as _;

use crate::platform::{RecordOutcome, ShortcutRecorder, WinShortcutRecorder};

/// 组合键在设置文件中的 key 前缀。
const SETTINGS_KEY_PREFIX: &str = "shortcut.";
/// 前端共享的设置文件（plugin-store）。
const SETTINGS_FILE: &str = "settings.json";

/// 可改绑的全局快捷键动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShortcutAction {
    Clipboard,
    Palette,
    Translate,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 3] = [Self::Clipboard, Self::Palette, Self::Translate];

    /// 动作的稳定标识：设置文件 key 与 serde 序列化共用同一套词。
    fn slug(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Palette => "palette",
            Self::Translate => "translate",
        }
    }

    /// 用户可见名称（错误提示里指明冲突对象）。
    fn label(self) -> &'static str {
        match self {
            Self::Clipboard => "剪贴板浮层",
            Self::Palette => "命令面板",
            Self::Translate => "划词翻译",
        }
    }

    fn settings_key(self) -> String {
        format!("{SETTINGS_KEY_PREFIX}{}", self.slug())
    }
}

/// 各动作的默认组合键（插件解析格式）。默认 Alt+V 而非 Win+V：Win+V 被
/// 系统剪贴板历史固定占用。
const DEFAULT_COMBOS: [(ShortcutAction, &str); 3] = [
    (ShortcutAction::Clipboard, "alt+v"),
    (ShortcutAction::Palette, "super+alt+space"),
    (ShortcutAction::Translate, "alt+t"),
];

/// 注册 seam：对 OS 注册/注销组合键。真实实现走 global-shortcut 插件，
/// 测试用假实现模拟系统级占用。
trait ShortcutRegistrar {
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut) -> Result<(), String>;
}

/// 持久化 seam：组合键字符串按动作存取。真实实现走设置文件，测试用内存表。
trait ShortcutSettings {
    fn get(&self, action: ShortcutAction) -> Option<String>;
    fn set(&self, action: ShortcutAction, combo: &str) -> Result<(), String>;
}

/// 真实注册器：经 global-shortcut 插件对 OS 注册。
struct PluginRegistrar<'a>(&'a AppHandle);

impl ShortcutRegistrar for PluginRegistrar<'_> {
    fn register(&self, shortcut: Shortcut) -> Result<(), String> {
        self.0
            .global_shortcut()
            .register(shortcut)
            .map_err(|e| e.to_string())
    }

    fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
        self.0
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|e| e.to_string())
    }
}

/// 真实持久化：与前端共享的设置文件。
struct SettingsStore<'a>(&'a AppHandle);

impl ShortcutSettings for SettingsStore<'_> {
    fn get(&self, action: ShortcutAction) -> Option<String> {
        let store = self.0.store(SETTINGS_FILE).ok()?;
        store.get(action.settings_key())?.as_str().map(String::from)
    }

    fn set(&self, action: ShortcutAction, combo: &str) -> Result<(), String> {
        let store = self.0.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
        store.set(action.settings_key(), combo);
        store.save().map_err(|e| e.to_string())
    }
}

/// 动作的一条绑定：组合键 + OS 注册是否实际生效。注册状态如实记录，让
/// 启动期注册失败（被系统占用）在设置页可见，且重试路径有据可依。
#[derive(Clone, Copy)]
struct Binding {
    shortcut: Shortcut,
    registered: bool,
}

/// 动作 → 当前绑定。呼出分发与改键命令共享。
pub struct ShortcutBindings {
    bindings: Mutex<HashMap<ShortcutAction, Binding>>,
}

/// 当前绑定（设置页展示用）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub combo: String,
    pub registered: bool,
}

impl ShortcutBindings {
    fn new(bindings: HashMap<ShortcutAction, Binding>) -> Self {
        Self {
            bindings: Mutex::new(bindings),
        }
    }

    /// 组合键当前绑定的动作；未绑定返回 None。
    pub fn action_of(&self, shortcut: Shortcut) -> Option<ShortcutAction> {
        self.bindings
            .lock()
            .unwrap()
            .iter()
            .find(|(_, &bound)| bound.shortcut == shortcut)
            .map(|(&action, _)| action)
    }

    fn list(&self) -> Vec<ShortcutBinding> {
        let bindings = self.bindings.lock().unwrap();
        ShortcutAction::ALL
            .into_iter()
            .filter_map(|action| {
                let bound = bindings.get(&action)?;
                Some(ShortcutBinding {
                    action,
                    combo: combo_string(bound.shortcut),
                    registered: bound.registered,
                })
            })
            .collect()
    }

    /// 改绑：走完整校验-注册-回滚-落盘流程。
    fn set(&self, app: &AppHandle, action: ShortcutAction, combo: &str) -> Result<(), String> {
        let registrar = PluginRegistrar(app);
        let settings = SettingsStore(app);
        let mut bindings = self.bindings.lock().unwrap();
        set_binding(&mut bindings, &registrar, &settings, action, combo)
    }
}

/// 解析组合键字符串（插件解析格式），并要求至少含一个修饰键（Ctrl / Alt /
/// Win 之一）；Shift 不算。
pub(crate) fn parse_combo(combo: &str) -> Result<Shortcut, String> {
    let shortcut =
        Shortcut::from_str(combo).map_err(|e| format!("无法识别的组合键“{combo}”：{e}"))?;
    if !shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err("组合键必须包含 Ctrl、Alt 或 Win 中的至少一个修饰键".into());
    }
    Ok(shortcut)
}

/// 组合键的规范字符串：修饰键按 ctrl, alt, shift, super 排列，主键为
/// KeyboardEvent.code 命名（如 "ctrl+alt+KeyV"），与前端 eventToCombo 一致。
fn combo_string(shortcut: Shortcut) -> String {
    let mut combo = String::new();
    for (name, flag) in [
        ("ctrl", Modifiers::CONTROL),
        ("alt", Modifiers::ALT),
        ("shift", Modifiers::SHIFT),
        ("super", Modifiers::SUPER),
    ] {
        if shortcut.mods.contains(flag) {
            combo.push_str(name);
            combo.push('+');
        }
    }
    combo.push_str(&shortcut.key.to_string());
    combo
}

/// 目标动作之外是否有其他动作绑定了同一组合键（Youmi 内部互斥）。
fn conflicting(
    bindings: &HashMap<ShortcutAction, Binding>,
    action: ShortcutAction,
    shortcut: Shortcut,
) -> Option<ShortcutAction> {
    ShortcutAction::ALL.into_iter().find(|&other| {
        other != action && bindings.get(&other).is_some_and(|b| b.shortcut == shortcut)
    })
}

/// 注册失败的提示：系统级占用只能靠注册失败感知；Win+V 被系统剪贴板历史
/// 固定占用是已知案例，命中时点名。`kept` = 旧绑定是否仍在 OS 上生效。
fn registration_error(combo: &str, shortcut: Shortcut, err: &str, kept: bool) -> String {
    let known = if shortcut.mods.contains(Modifiers::SUPER) && shortcut.key == Code::KeyV {
        "（已知冲突：Win+V 被系统剪贴板历史固定占用）"
    } else {
        ""
    };
    let restore = if kept {
        "原绑定保持不变".to_string()
    } else {
        "该工具的组合键当前未生效，可重试或换一个组合".to_string()
    };
    format!("「{combo}」注册失败：可能已被系统或其他程序占用{known}；{restore}（{err}）")
}

/// 改绑流程：解析与互斥校验失败时不触碰任何状态；注册成功才落盘并更新
/// 绑定表；注册失败回滚旧注册并报错。与现绑定相同且已注册时幂等成功；
/// 启动期注册失败的绑定（未注册）重输同组合会真正重试注册。
fn set_binding(
    bindings: &mut HashMap<ShortcutAction, Binding>,
    registrar: &dyn ShortcutRegistrar,
    settings: &dyn ShortcutSettings,
    action: ShortcutAction,
    combo: &str,
) -> Result<(), String> {
    let combo = combo.trim();
    let shortcut = parse_combo(combo)?;
    if let Some(other) = conflicting(bindings, action, shortcut) {
        return Err(format!(
            "「{combo}」已被{}占用，一个组合键只能绑定一个工具",
            other.label()
        ));
    }
    let old = bindings.get(&action).copied();
    if let Some(old) = old {
        if old.shortcut == shortcut {
            if old.registered {
                return Ok(());
            }
            return match registrar.register(shortcut) {
                Ok(()) => {
                    bindings.insert(
                        action,
                        Binding {
                            shortcut,
                            registered: true,
                        },
                    );
                    Ok(())
                }
                Err(e) => Err(registration_error(combo, shortcut, &e, false)),
            };
        }
        if old.registered {
            // 尽力注销：注销失败不阻塞新组合键注册
            if let Err(e) = registrar.unregister(old.shortcut) {
                eprintln!("youmi: 注销 {} 旧组合键失败: {e}", action.label());
            }
        }
    }
    if let Err(e) = registrar.register(shortcut) {
        // 回滚旧注册；回滚也失败时如实降级注册状态，绑定表与 OS 保持一致
        let kept = match old {
            Some(old) if old.registered => {
                let restored = registrar.register(old.shortcut).is_ok();
                if !restored {
                    if let Some(entry) = bindings.get_mut(&action) {
                        entry.registered = false;
                    }
                    eprintln!("youmi: 回滚 {} 旧组合键注册失败", action.label());
                }
                restored
            }
            _ => false,
        };
        return Err(registration_error(combo, shortcut, &e, kept));
    }
    bindings.insert(
        action,
        Binding {
            shortcut,
            registered: true,
        },
    );
    if let Err(e) = settings.set(action, combo) {
        // 注册已生效、绑定表已更新：落盘失败只影响重启后的保留
        eprintln!("youmi: 保存 {} 组合键到设置文件失败: {e}", action.label());
        return Err(format!("组合键已生效，但写入设置失败，重启后不会保留：{e}"));
    }
    Ok(())
}

/// 启动期单动作绑定解析：存储组合键非法时回退默认并告警；与已绑定动作
/// 冲突时也回退默认，默认仍冲突则放弃（该动作本次启动不生效，日志可见）。
fn startup_binding(
    stored: Option<String>,
    default_combo: &str,
    bindings: &HashMap<ShortcutAction, Binding>,
    action: ShortcutAction,
) -> Option<Shortcut> {
    let combo = stored.unwrap_or_else(|| default_combo.into());
    let shortcut = match parse_combo(&combo) {
        Ok(shortcut) => shortcut,
        Err(e) => {
            eprintln!(
                "youmi: {} 组合键“{combo}”非法（{e}），回退默认 {default_combo}",
                action.label()
            );
            parse_combo(default_combo).expect("默认组合键合法")
        }
    };
    if let Some(other) = conflicting(bindings, action, shortcut) {
        eprintln!(
            "youmi: {} 组合键与{}冲突，回退默认 {default_combo}",
            action.label(),
            other.label()
        );
        let default = parse_combo(default_combo).expect("默认组合键合法");
        if conflicting(bindings, action, default).is_some() {
            eprintln!("youmi: {} 默认组合键仍冲突，本次启动不生效", action.label());
            return None;
        }
        return Some(default);
    }
    Some(shortcut)
}

/// 动作分发：呼出该动作的全局工具。
pub fn activate(app: &AppHandle, action: ShortcutAction) {
    match action {
        ShortcutAction::Clipboard => {
            // 呼起前捕获前台窗口为回帖目标；收起分支（浮层自己在前台）捕获
            // 结果为无目标，恰好清掉过期目标
            crate::clipboard::pasteback::capture_current_target(app);
            if let Err(e) = crate::overlay::toggle(app, crate::CLIPBOARD_OVERLAY_LABEL) {
                eprintln!("youmi: 呼出剪贴板浮层失败: {e}");
            }
        }
        ShortcutAction::Palette => {
            if let Err(e) = crate::overlay::toggle(app, crate::palette::LABEL) {
                eprintln!("youmi: 呼出命令面板失败: {e}");
            }
            // 清空上次查询走呼起事件而非 focus：浮层共存后焦点从兄弟浮层回落
            // 同样触发 focus，那里清空会丢掉回填内容与上下文；收起分支发出的事件
            // 恰好为下次召起复位
            let _ = app.emit_to(crate::palette::LABEL, crate::palette::RESET_EVENT, ());
        }
        ShortcutAction::Translate => {
            // 取词必须先于显示（Ctrl+C 发给呼起时仍在前台的目标应用），
            // 编排含键击注入与轮询等待：放阻塞线程池，不占主线程
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                crate::translate::capture_and_show(&app);
            });
        }
    }
}

/// 启动：从设置读取各动作组合键并注册（非法/冲突回退默认），注册失败只
/// 降级日志不阻塞启动——绑定如实标记未注册，呼不出对应工具时用户可在
/// 设置页看到并改键。
pub fn setup(app: &mut tauri::App) {
    let settings = SettingsStore(app.handle());
    let registrar = PluginRegistrar(app.handle());
    let mut bindings: HashMap<ShortcutAction, Binding> = HashMap::new();
    for (action, default_combo) in DEFAULT_COMBOS {
        let stored = settings.get(action);
        let Some(shortcut) = startup_binding(stored, default_combo, &bindings, action) else {
            continue;
        };
        let registered = match registrar.register(shortcut) {
            Ok(()) => true,
            Err(e) => {
                eprintln!(
                    "youmi: 注册 {} 快捷键失败（可能被其他程序占用）: {e}",
                    action.label()
                );
                false
            }
        };
        bindings.insert(
            action,
            Binding {
                shortcut,
                registered,
            },
        );
    }
    app.manage(ShortcutBindings::new(bindings));
    app.manage(RecordState {
        recorder: Arc::new(WinShortcutRecorder),
    });
}

/// 当前各动作的组合键（设置页展示用）。
#[tauri::command]
pub fn shortcut_list(state: tauri::State<'_, ShortcutBindings>) -> Vec<ShortcutBinding> {
    state.list()
}

/// 改绑动作的组合键并立即生效；校验失败/内部冲突/注册失败时原绑定不变。
#[tauri::command]
pub fn shortcut_set(
    app: AppHandle,
    state: tauri::State<'_, ShortcutBindings>,
    action: ShortcutAction,
    combo: String,
) -> Result<(), String> {
    state.set(&app, action, &combo)
}

/// 录制事件：发往主窗口，payload 见 [`RecordEventPayload`]。
pub const RECORD_EVENT: &str = "shortcut-record";

/// 录制结果事件载荷：`combo` = 系统级截获的组合键规范串；Esc/主动取消为
/// cancelled，超时未截获为 timeout。互斥由类型表达，不发双空载荷。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RecordEventPayload {
    Recorded { combo: String },
    Cancelled,
    Timeout,
}

/// 改键录制的平台 seam。
pub struct RecordState {
    recorder: Arc<dyn ShortcutRecorder>,
}

fn record_sink(app: AppHandle) -> Arc<dyn Fn(RecordOutcome) + Send + Sync> {
    Arc::new(move |outcome| {
        let payload = match outcome {
            RecordOutcome::Recorded(combo) => RecordEventPayload::Recorded { combo },
            RecordOutcome::Cancelled => RecordEventPayload::Cancelled,
            RecordOutcome::Timeout => RecordEventPayload::Timeout,
        };
        if let Err(e) = app.emit_to(crate::MAIN_WINDOW_LABEL, RECORD_EVENT, payload) {
            eprintln!("youmi: 发送录制结果事件失败: {e}");
        }
    })
}

/// 开始一次性组合键录制。已在录制中时重启会话（旧会话的取消经事件通知，
/// 旧录制行随之退出录制态）。
#[tauri::command]
pub fn shortcut_record_begin(
    record: tauri::State<'_, RecordState>,
    app: AppHandle,
) -> Result<(), String> {
    if record.recorder.begin(record_sink(app.clone())).is_err() {
        record.recorder.cancel();
        record
            .recorder
            .begin(record_sink(app))
            .map_err(|e| format!("开始录制失败：{e}"))?;
    }
    Ok(())
}

/// 提前结束当前录制（用户点取消时调用）；不在录制中为空操作。
#[tauri::command]
pub fn shortcut_record_cancel(record: tauri::State<'_, RecordState>) {
    record.recorder.cancel();
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// 假注册器：`fail_register` 命中的组合键注册失败（模拟系统级占用），
    /// 其余成功并记录；注销总是成功并记录。调用序列供断言回滚行为。
    struct FakeRegistrar {
        fail_register: HashSet<Shortcut>,
        events: Mutex<Vec<String>>,
    }

    impl FakeRegistrar {
        fn new(fail_register: &[&str]) -> Self {
            Self {
                fail_register: fail_register
                    .iter()
                    .map(|combo| Shortcut::from_str(combo).unwrap())
                    .collect(),
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl ShortcutRegistrar for FakeRegistrar {
        fn register(&self, shortcut: Shortcut) -> Result<(), String> {
            let mut events = self.events.lock().unwrap();
            if self.fail_register.contains(&shortcut) {
                events.push(format!("register-fail:{}", combo_string(shortcut)));
                return Err("os: already registered by others".into());
            }
            events.push(format!("register:{}", combo_string(shortcut)));
            Ok(())
        }

        fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
            self.events
                .lock()
                .unwrap()
                .push(format!("unregister:{}", combo_string(shortcut)));
            Ok(())
        }
    }

    /// 假持久化：内存键值表。
    struct FakeSettings(Mutex<HashMap<ShortcutAction, String>>);

    impl FakeSettings {
        fn new() -> Self {
            Self(Mutex::new(HashMap::new()))
        }

        fn get_raw(&self, action: ShortcutAction) -> Option<String> {
            self.0.lock().unwrap().get(&action).cloned()
        }
    }

    impl ShortcutSettings for FakeSettings {
        fn get(&self, action: ShortcutAction) -> Option<String> {
            self.get_raw(action)
        }

        fn set(&self, action: ShortcutAction, combo: &str) -> Result<(), String> {
            self.0.lock().unwrap().insert(action, combo.to_string());
            Ok(())
        }
    }

    fn binding(combo: &str) -> Binding {
        Binding {
            shortcut: Shortcut::from_str(combo).unwrap(),
            registered: true,
        }
    }

    /// 未注册绑定（模拟启动期注册失败）。
    fn unregistered(combo: &str) -> Binding {
        Binding {
            shortcut: Shortcut::from_str(combo).unwrap(),
            registered: false,
        }
    }

    /// 三动作默认绑定的初始状态（全部已注册），与启动时一致。
    fn initial_bindings() -> HashMap<ShortcutAction, Binding> {
        DEFAULT_COMBOS
            .into_iter()
            .map(|(action, combo)| (action, binding(combo)))
            .collect()
    }

    fn bound_shortcut(
        bindings: &HashMap<ShortcutAction, Binding>,
        action: ShortcutAction,
    ) -> Shortcut {
        bindings[&action].shortcut
    }

    fn set_ok(
        bindings: &mut HashMap<ShortcutAction, Binding>,
        registrar: &FakeRegistrar,
        settings: &FakeSettings,
        action: ShortcutAction,
        combo: &str,
    ) {
        set_binding(bindings, registrar, settings, action, combo).unwrap();
    }

    #[test]
    fn parse_combo_accepts_modifier_combos() {
        assert!(parse_combo("alt+v").is_ok());
        assert!(parse_combo("ctrl+shift+v").is_ok());
        assert!(parse_combo("super+alt+space").is_ok());
    }

    #[test]
    fn parse_combo_rejects_unparseable_string() {
        let err = parse_combo("不合法").unwrap_err();
        assert!(err.contains("无法识别的组合键"));
    }

    #[test]
    fn parse_combo_rejects_missing_required_modifier() {
        for combo in ["v", "shift+v", "f5"] {
            let err = parse_combo(combo).unwrap_err();
            assert!(
                err.contains("Ctrl、Alt 或 Win"),
                "combo {combo} error: {err}"
            );
        }
    }

    #[test]
    fn rebind_success_updates_registration_settings_and_bindings() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&[]);
        let settings = FakeSettings::new();

        set_ok(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "ctrl+shift+c",
        );

        assert_eq!(
            registrar.events(),
            vec!["unregister:alt+KeyV", "register:ctrl+shift+KeyC"]
        );
        assert_eq!(
            settings.get_raw(ShortcutAction::Clipboard),
            Some("ctrl+shift+c".into())
        );
        assert_eq!(
            bound_shortcut(&bindings, ShortcutAction::Clipboard),
            Shortcut::from_str("ctrl+shift+c").unwrap()
        );
        assert!(bindings[&ShortcutAction::Clipboard].registered);
    }

    #[test]
    fn rebind_rejects_combo_bound_to_another_action() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&[]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Translate,
            "alt+v",
        )
        .unwrap_err();

        assert!(err.contains("剪贴板浮层"), "error: {err}");
        // 校验失败：OS 与持久化都不被触碰
        assert!(registrar.events().is_empty());
        assert_eq!(settings.get_raw(ShortcutAction::Translate), None);
        assert_eq!(
            bound_shortcut(&bindings, ShortcutAction::Translate),
            Shortcut::from_str("alt+t").unwrap()
        );
    }

    #[test]
    fn rebind_rejects_combo_without_required_modifier() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&[]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "shift+v",
        )
        .unwrap_err();

        assert!(err.contains("Ctrl、Alt 或 Win"), "error: {err}");
        assert!(registrar.events().is_empty());
        assert_eq!(settings.get_raw(ShortcutAction::Clipboard), None);
    }

    #[test]
    fn rebind_register_failure_rolls_back_to_old_binding() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&["ctrl+j"]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "ctrl+j",
        )
        .unwrap_err();

        assert!(err.contains("注册失败"), "error: {err}");
        assert!(err.contains("原绑定保持不变"), "error: {err}");
        // 旧组合键先注销、失败后回滚注册
        assert_eq!(
            registrar.events(),
            vec![
                "unregister:alt+KeyV",
                "register-fail:ctrl+KeyJ",
                "register:alt+KeyV"
            ]
        );
        assert_eq!(
            bound_shortcut(&bindings, ShortcutAction::Clipboard),
            Shortcut::from_str("alt+v").unwrap()
        );
        assert!(bindings[&ShortcutAction::Clipboard].registered);
        assert_eq!(settings.get_raw(ShortcutAction::Clipboard), None);
    }

    #[test]
    fn rebind_register_failure_with_rollback_failure_marks_unregistered() {
        let mut bindings = initial_bindings();
        // 新旧组合键都注册失败：回滚无效，注册状态如实降级
        let registrar = FakeRegistrar::new(&["ctrl+j", "alt+v"]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "ctrl+j",
        )
        .unwrap_err();

        assert!(err.contains("当前未生效"), "error: {err}");
        assert!(!bindings[&ShortcutAction::Clipboard].registered);
    }

    #[test]
    fn rebind_register_failure_names_known_win_v_conflict() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&["super+v"]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "super+v",
        )
        .unwrap_err();

        assert!(err.contains("系统剪贴板历史"), "error: {err}");
    }

    #[test]
    fn rebind_to_registered_same_combo_is_idempotent_no_op() {
        let mut bindings = initial_bindings();
        let registrar = FakeRegistrar::new(&[]);
        let settings = FakeSettings::new();

        set_ok(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            " alt+v ",
        );

        assert!(registrar.events().is_empty());
        assert_eq!(settings.get_raw(ShortcutAction::Clipboard), None);
    }

    #[test]
    fn rebind_to_unregistered_same_combo_retries_registration() {
        let mut bindings = initial_bindings();
        bindings.insert(ShortcutAction::Clipboard, unregistered("alt+v"));
        let registrar = FakeRegistrar::new(&[]);
        let settings = FakeSettings::new();

        set_ok(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "alt+v",
        );

        // 未注册绑定重输同组合：真正重试注册并恢复注册状态
        assert_eq!(registrar.events(), vec!["register:alt+KeyV"]);
        assert!(bindings[&ShortcutAction::Clipboard].registered);
    }

    #[test]
    fn rebind_to_unregistered_same_combo_retry_failure_reports_honestly() {
        let mut bindings = initial_bindings();
        bindings.insert(ShortcutAction::Clipboard, unregistered("alt+v"));
        let registrar = FakeRegistrar::new(&["alt+v"]);
        let settings = FakeSettings::new();

        let err = set_binding(
            &mut bindings,
            &registrar,
            &settings,
            ShortcutAction::Clipboard,
            "alt+v",
        )
        .unwrap_err();

        assert!(err.contains("当前未生效"), "error: {err}");
        assert!(!bindings[&ShortcutAction::Clipboard].registered);
    }

    #[test]
    fn conflicting_finds_other_action_only() {
        let bindings = initial_bindings();
        let alt_v = Shortcut::from_str("alt+v").unwrap();
        assert_eq!(
            conflicting(&bindings, ShortcutAction::Clipboard, alt_v),
            None
        );
        assert_eq!(
            conflicting(&bindings, ShortcutAction::Translate, alt_v),
            Some(ShortcutAction::Clipboard)
        );
    }

    #[test]
    fn combo_string_is_canonical_and_frontend_aligned() {
        let canonical = |combo: &str| combo_string(parse_combo(combo).unwrap());
        assert_eq!(canonical("alt+v"), "alt+KeyV");
        // 修饰键固定按 ctrl, alt, shift, super 排列，与输入顺序无关
        assert_eq!(canonical("super+alt+space"), "alt+super+Space");
        assert_eq!(canonical("shift+ctrl+c"), "ctrl+shift+KeyC");
        assert_eq!(canonical("alt+1"), "alt+Digit1");
    }

    #[test]
    fn startup_binding_keeps_valid_stored_combo() {
        let bindings = initial_bindings();
        let shortcut = startup_binding(
            Some("ctrl+shift+c".into()),
            "alt+v",
            &bindings,
            ShortcutAction::Clipboard,
        )
        .unwrap();
        assert_eq!(shortcut, Shortcut::from_str("ctrl+shift+c").unwrap());
    }

    #[test]
    fn startup_binding_falls_back_to_default_on_invalid_stored() {
        let bindings = initial_bindings();
        let shortcut = startup_binding(
            Some("不合法".into()),
            "alt+v",
            &bindings,
            ShortcutAction::Clipboard,
        )
        .unwrap();
        assert_eq!(shortcut, Shortcut::from_str("alt+v").unwrap());
    }

    #[test]
    fn startup_binding_falls_back_to_default_on_internal_conflict() {
        let bindings = initial_bindings();
        // 手改设置让翻译与剪贴板同键：回退翻译的默认 alt+t
        let shortcut = startup_binding(
            Some("alt+v".into()),
            "alt+t",
            &bindings,
            ShortcutAction::Translate,
        )
        .unwrap();
        assert_eq!(shortcut, Shortcut::from_str("alt+t").unwrap());
    }

    #[test]
    fn startup_binding_gives_up_when_default_conflicts_too() {
        let bindings = initial_bindings();
        // 存储值与默认值都撞已绑定的 alt+v：该动作本次启动不生效
        let shortcut = startup_binding(
            Some("alt+v".into()),
            "alt+v",
            &bindings,
            ShortcutAction::Translate,
        );
        assert_eq!(shortcut, None);
    }

    #[test]
    fn list_reports_registration_state() {
        let mut bindings = initial_bindings();
        bindings.insert(ShortcutAction::Clipboard, unregistered("alt+v"));
        let state = ShortcutBindings::new(bindings);

        let listed = state.list();

        let clipboard = listed
            .iter()
            .find(|b| b.action == ShortcutAction::Clipboard)
            .unwrap();
        assert_eq!(clipboard.combo, "alt+KeyV");
        assert!(!clipboard.registered);
        let translate = listed
            .iter()
            .find(|b| b.action == ShortcutAction::Translate)
            .unwrap();
        assert!(translate.registered);
    }

    #[test]
    fn action_serde_and_settings_key_share_slug() {
        assert_eq!(
            serde_json::to_value(ShortcutAction::Clipboard).unwrap(),
            "clipboard"
        );
        assert_eq!(
            serde_json::from_value::<ShortcutAction>(serde_json::json!("translate")).unwrap(),
            ShortcutAction::Translate
        );
        assert_eq!(ShortcutAction::Palette.settings_key(), "shortcut.palette");
    }
}
