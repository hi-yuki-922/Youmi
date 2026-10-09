pub mod clipboard;
pub mod db;
pub mod platform;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use clipboard::commands::ClipboardState;
use clipboard::engine::ClipboardEngine;
use clipboard::repository::ClipboardHistoryRepository;
use platform::{ClipboardAccess, ClipboardWatcher};

const MAIN_WINDOW_LABEL: &str = "main";

fn show_main_window(app: &tauri::AppHandle) {
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
        .setup(|app| {
            setup_tray(app)?;
            setup_clipboard(app);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
