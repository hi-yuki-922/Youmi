pub mod clipboard;
pub mod db;
pub mod overlay;
pub mod palette;
pub mod platform;

#[cfg(test)]
mod test_support;

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt as _, Shortcut, ShortcutState};

use clipboard::commands::ClipboardState;
use clipboard::engine::ClipboardEngine;
use clipboard::pasteback::PastebackState;
use clipboard::repository::ClipboardHistoryRepository;
use overlay::OverlaySize;
use platform::{ClipboardAccess, ClipboardWatcher};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const CLIPBOARD_OVERLAY_LABEL: &str = "clipboard-overlay";
/// 剪贴板浮层呼出/收起的全局快捷键，重启后保持不变。
const CLIPBOARD_OVERLAY_HOTKEY: &str = "alt+v";
/// 剪贴板浮层尺寸（逻辑像素）。
const CLIPBOARD_OVERLAY_SIZE: OverlaySize = OverlaySize {
    width: 720.0,
    height: 560.0,
};
/// 命令面板呼出/收起的全局快捷键，重启后保持不变。
const PALETTE_HOTKEY: &str = "super+alt+space";
/// 命令面板尺寸（逻辑像素）：单输入框 + 条目列表的轻量形态，比剪贴板浮层小。
const PALETTE_SIZE: OverlaySize = OverlaySize {
    width: 640.0,
    height: 420.0,
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

fn setup_clipboard_overlay(app: &tauri::App, hotkey: Shortcut) {
    if let Err(e) = overlay::create(
        app.app_handle(),
        CLIPBOARD_OVERLAY_LABEL,
        "/overlay/clipboard",
        "Youmi 剪贴板",
        CLIPBOARD_OVERLAY_SIZE,
    ) {
        eprintln!("youmi: 剪贴板浮层窗口创建失败: {e}");
        return;
    }
    // 快捷键被其他程序占用时注册失败：仅降级日志，不阻塞应用启动
    if let Err(e) = app.global_shortcut().register(hotkey) {
        eprintln!("youmi: 注册 {CLIPBOARD_OVERLAY_HOTKEY} 失败（可能被其他程序占用）: {e}");
    }
}

fn setup_palette_overlay(app: &tauri::App, hotkey: Shortcut) {
    if let Err(e) = overlay::create(
        app.app_handle(),
        palette::LABEL,
        "/overlay/palette",
        "Youmi 命令面板",
        PALETTE_SIZE,
    ) {
        eprintln!("youmi: 命令面板浮层窗口创建失败: {e}");
        return;
    }
    // 快捷键被其他程序占用时注册失败：仅降级日志，不阻塞应用启动
    if let Err(e) = app.global_shortcut().register(hotkey) {
        eprintln!("youmi: 注册 {PALETTE_HOTKEY} 失败（可能被其他程序占用）: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let clipboard_hotkey: Shortcut = CLIPBOARD_OVERLAY_HOTKEY
        .parse()
        .expect("内置快捷键字符串合法");
    let palette_hotkey: Shortcut = PALETTE_HOTKEY.parse().expect("内置快捷键字符串合法");

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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if *shortcut == clipboard_hotkey {
                        // 呼起前捕获前台窗口为回帖目标；
                        // 收起分支（浮层自己在前台）捕获结果为无目标，恰好清掉过期目标
                        clipboard::pasteback::capture_current_target(app);
                        if let Err(e) = overlay::toggle(app, CLIPBOARD_OVERLAY_LABEL) {
                            eprintln!("youmi: 呼出剪贴板浮层失败: {e}");
                        }
                    } else if *shortcut == palette_hotkey {
                        if let Err(e) = overlay::toggle(app, palette::LABEL) {
                            eprintln!("youmi: 呼出命令面板失败: {e}");
                        }
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
            setup_tray(app)?;
            setup_clipboard(app);
            setup_clipboard_overlay(app, clipboard_hotkey);
            setup_palette_overlay(app, palette_hotkey);
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
            palette::palette_open_tool,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
