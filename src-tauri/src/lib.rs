pub mod apps;
pub mod clipboard;
pub mod db;
pub mod overlay;
pub mod palette;
pub mod platform;
pub mod shortcuts;
pub mod translate;
pub mod vault;

#[cfg(test)]
mod test_support;

use std::sync::Arc;
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter as _, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::ShortcutState;

use clipboard::commands::ClipboardState;
use clipboard::engine::ClipboardEngine;
use clipboard::pasteback::PastebackState;
use clipboard::repository::ClipboardHistoryRepository;
use overlay::OverlaySize;
use platform::{ClipboardAccess, ClipboardWatcher};
use shortcuts::ShortcutBindings;
use translate::SecretState;
use vault::VaultState;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const CLIPBOARD_OVERLAY_LABEL: &str = "clipboard-overlay";
/// 剪贴板浮层尺寸（逻辑像素）。
const CLIPBOARD_OVERLAY_SIZE: OverlaySize = OverlaySize {
    width: 720.0,
    height: 560.0,
};
/// 命令面板尺寸（逻辑像素）：单输入框 + 条目列表的轻量形态，比剪贴板浮层小。
const PALETTE_SIZE: OverlaySize = OverlaySize {
    width: 640.0,
    height: 420.0,
};
/// 划词翻译浮层尺寸（逻辑像素）：小浮层贴近光标显示。
const TRANSLATE_OVERLAY_SIZE: OverlaySize = OverlaySize {
    width: 460.0,
    height: 360.0,
};

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    let mut tray = TrayIconBuilder::with_id("youmi-tray")
        .tooltip("Youmi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

/// 剪贴板引擎初始化：数据库 + engine + 监听线程。失败仅日志降级，不阻塞应用启动。
fn setup_clipboard(app: &tauri::App) {
    let init = || -> Result<(), Box<dyn std::error::Error>> {
        let db_path = app.path().app_data_dir()?.join("youmi.db");
        if let Some(dir) = db_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let repo = Arc::new(ClipboardHistoryRepository::open(&db_path)?);
        let access: Arc<dyn ClipboardAccess> = Arc::new(platform::WinClipboardAccess);
        let engine = Arc::new(ClipboardEngine::new(Arc::clone(&repo), access));

        app.manage(ClipboardState {
            engine: Arc::clone(&engine),
            repo,
        });
        app.manage(PastebackState::new(Arc::new(platform::WinPastebackAccess)));

        let watcher = platform::WinClipboardWatcher;
        std::thread::Builder::new()
            .name("clipboard-watch".into())
            .spawn(move || {
                // handle_change 内部自带降级，监听线程错误只在退出时打日志
                if let Err(e) = watcher.watch(Box::new(move || engine.handle_change())) {
                    eprintln!("youmi: 剪贴板监听线程退出: {e}");
                }
            })?;
        Ok(())
    };

    if let Err(e) = init() {
        eprintln!("youmi: 剪贴板引擎初始化失败（历史不可用）: {e}");
    }
}

fn setup_clipboard_overlay(app: &tauri::App) {
    if let Err(e) = overlay::create(
        app.app_handle(),
        CLIPBOARD_OVERLAY_LABEL,
        "/overlay/clipboard",
        "Youmi 剪贴板",
        CLIPBOARD_OVERLAY_SIZE,
    ) {
        eprintln!("youmi: 剪贴板浮层窗口创建失败: {e}");
    }
}

/// 应用索引：后台线程首扫开始菜单，不阻塞启动；呼起面板时按需刷新。
fn setup_apps(app: &tauri::App) {
    let state = apps::AppsState::default();
    apps::ensure_fresh(&state);
    app.manage(state);
}

/// 密码本：与剪贴板历史共用数据库文件（各自的连接）。剪贴板引擎不可用时
/// 密码本照常初始化，仅复制功能降级报错。
fn setup_vault(app: &tauri::App) {
    let init = || -> Result<(), Box<dyn std::error::Error>> {
        let db_path = app.path().app_data_dir()?.join("youmi.db");
        let repo = Arc::new(vault::repository::VaultRepository::open(&db_path)?);
        let engine = app
            .try_state::<ClipboardState>()
            .map(|s| Arc::clone(&s.engine));
        app.manage(VaultState::new(repo, engine));
        Ok(())
    };

    if let Err(e) = init() {
        eprintln!("youmi: 密码本初始化失败（密码本不可用）: {e}");
    }
}

/// 自动锁定哨兵：解锁会话到期（无操作超过锁定超时）即落锁并通知主窗口，
/// 前端收到事件切回解锁界面。命令路径自带到期检查，哨兵负责界面联动。
fn spawn_vault_autolock(app: tauri::AppHandle) {
    let spawned = std::thread::Builder::new()
        .name("vault-autolock".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if let Some(state) = app.try_state::<VaultState>() {
                if state.expire_if_due() {
                    let _ = app.emit_to(MAIN_WINDOW_LABEL, vault::LOCKED_EVENT, ());
                }
            }
        });
    if let Err(e) = spawned {
        eprintln!("youmi: 密码本自动锁定线程启动失败（仅失去界面联动）: {e}");
    }
}

fn setup_palette_overlay(app: &tauri::App) {
    if let Err(e) = overlay::create(
        app.app_handle(),
        palette::LABEL,
        "/overlay/palette",
        "Youmi 命令面板",
        PALETTE_SIZE,
    ) {
        eprintln!("youmi: 命令面板浮层窗口创建失败: {e}");
    }
}

/// 划词翻译：凭据存取独立管理（引擎初始化失败时设置页仍可用）；取词依赖
/// 剪贴板引擎的自家写入过滤，引擎缺失时热键降级为浮层内提示。
fn setup_translate_overlay(app: &tauri::App) {
    app.manage(SecretState {
        store: Arc::new(platform::WinSecretStore),
    });
    if let Err(e) = overlay::create(
        app.app_handle(),
        translate::LABEL,
        "/overlay/translate",
        "Youmi 划词翻译",
        TRANSLATE_OVERLAY_SIZE,
    ) {
        eprintln!("youmi: 划词翻译浮层窗口创建失败: {e}");
        return;
    }
    if let Some(state) = app.try_state::<ClipboardState>() {
        app.manage(translate::TranslateState {
            engine: Arc::clone(&state.engine),
            selection: Arc::new(platform::WinSelectionAccess),
        });
    } else {
        eprintln!("youmi: 剪贴板引擎未初始化，划词取词不可用");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance 必须最先注册：二次启动立即退出，并把唤起意图交给已有实例
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // 翻译请求经前端自定义 fetch 走此插件，绕开 webview 的 CORS 限制
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    // 组合键 → 动作的映射在 ShortcutBindings 里，改键即时生效
                    let Some(bindings) = app.try_state::<ShortcutBindings>() else {
                        return;
                    };
                    if let Some(action) = bindings.action_of(*shortcut) {
                        shortcuts::activate(app, action);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 主窗口关闭按钮 = 隐藏到托盘，进程与托盘保持存活；真正退出走托盘菜单。
                // 其余窗口（未来浮层等）正常关闭。
                if window.label() == MAIN_WINDOW_LABEL {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(move |app| {
            // 固定浮层状态先于浮层窗口创建管理：失焦 handler 首次触发就能读到
            app.manage(overlay::PinnedOverlays::default());
            setup_tray(app)?;
            setup_clipboard(app);
            setup_vault(app);
            setup_apps(app);
            setup_clipboard_overlay(app);
            setup_palette_overlay(app);
            setup_translate_overlay(app);
            shortcuts::setup(app);
            spawn_vault_autolock(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            clipboard::commands::clipboard_query,
            clipboard::commands::clipboard_get_image,
            clipboard::commands::clipboard_delete,
            clipboard::commands::clipboard_clear,
            clipboard::commands::clipboard_set_capacity,
            clipboard::commands::clipboard_write_own_text,
            clipboard::commands::clipboard_write_own_image_png,
            clipboard::commands::clipboard_pasteback,
            vault::vault_status,
            vault::vault_setup,
            vault::vault_unlock,
            vault::vault_lock,
            vault::vault_set_lock_timeout,
            vault::vault_query,
            vault::vault_create,
            vault::vault_update,
            vault::vault_delete,
            vault::vault_copy,
            palette::palette_open_tool,
            apps::apps_query,
            apps::apps_launch,
            overlay::overlay_set_pinned,
            shortcuts::shortcut_list,
            shortcuts::shortcut_record_begin,
            shortcuts::shortcut_record_cancel,
            shortcuts::shortcut_set,
            translate::translate_has_api_key,
            translate::translate_get_api_key,
            translate::translate_set_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
