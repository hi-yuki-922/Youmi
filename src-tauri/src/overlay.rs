//! 浮层窗口模式：无边框、置顶、失焦即隐（hide 不销毁）、定位到鼠标所在显示器
//! 居中。剪贴板浮层、命令面板、划词翻译共用此实现。

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WindowEvent,
};

/// 浮层统一尺寸（逻辑像素）。
const OVERLAY_WIDTH: f64 = 720.0;
const OVERLAY_HEIGHT: f64 = 560.0;

/// 创建一个隐藏的浮层窗口并挂上失焦即隐。
/// 窗口以 hide 代替销毁，保证再次呼起瞬时。
///
/// 注意：WebView2 要求同一 user data folder 的所有 webview 使用完全一致的
/// browser args（实测：只给主窗口加 additionalBrowserArgs 会让隐藏的浮层
/// 创建报 0x8007139F INVALID_STATE）；新增浮层窗口保持默认参数即可。
pub fn create(app: &AppHandle, label: &str, path: &str, title: &str) -> tauri::Result<()> {
    let window = tauri::WebviewWindowBuilder::new(app, label, WebviewUrl::App(path.into()))
        .title(title)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
        .build()?;

    let owner = app.clone();
    let hidden_label = label.to_string();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            if let Some(window) = owner.get_webview_window(&hidden_label) {
                let _ = window.hide();
            }
        }
    });
    Ok(())
}

/// 呼起/隐藏浮层：可见且聚焦则隐藏，否则移到鼠标所在显示器居中并聚焦显示。
/// 可见但未聚焦（失焦 hide 与再次呼起的竞态窗口）按"呼起"处理。
pub fn toggle(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(()); // 浮层初始化失败的降级场景：呼不起但不崩溃
    };
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        window.hide()
    } else {
        move_to_cursor_monitor(app, &window);
        window.show()?;
        window.set_focus()
    }
}

/// 把窗口移到鼠标所在显示器居中；取不到光标/显示器时保持原位。
fn move_to_cursor_monitor(app: &AppHandle, window: &WebviewWindow) {
    let moved = (|| -> tauri::Result<()> {
        let cursor = app.cursor_position()?;
        let monitors = app.available_monitors()?;
        let Some(monitor) = monitors
            .iter()
            .find(|m| contains((*m.position(), *m.size()), cursor))
        else {
            return Ok(());
        };
        let size = window.outer_size()?;
        let x = monitor.position().x + (monitor.size().width as i32 - size.width as i32) / 2;
        let y = monitor.position().y + (monitor.size().height as i32 - size.height as i32) / 2;
        window.set_position(PhysicalPosition::new(x, y))
    })();
    if let Err(e) = moved {
        eprintln!("youmi: 浮层定位到鼠标所在显示器失败: {e}");
    }
}

/// 物理坐标意义上光标是否落在显示器内（右/下边界半开区间，多屏相邻不重叠判定）。
fn contains(
    monitor: (PhysicalPosition<i32>, PhysicalSize<u32>),
    cursor: PhysicalPosition<f64>,
) -> bool {
    let (position, size) = monitor;
    let max_x = f64::from(position.x + size.width as i32);
    let max_y = f64::from(position.y + size.height as i32);
    f64::from(position.x) <= cursor.x
        && cursor.x < max_x
        && f64::from(position.y) <= cursor.y
        && cursor.y < max_y
}

#[cfg(test)]
mod tests {
    use super::contains;
    use tauri::{PhysicalPosition, PhysicalSize};

    fn monitor(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        (
            PhysicalPosition::new(x, y),
            PhysicalSize::new(width, height),
        )
    }

    #[test]
    fn cursor_inside_monitor_is_contained() {
        let m = monitor(0, 0, 1920, 1080);
        assert!(contains(m, PhysicalPosition::new(0.0, 0.0)));
        assert!(contains(m, PhysicalPosition::new(1919.5, 1079.5)));
    }

    #[test]
    fn cursor_on_right_or_bottom_edge_belongs_to_next_monitor() {
        let m = monitor(0, 0, 1920, 1080);
        assert!(!contains(m, PhysicalPosition::new(1920.0, 0.0)));
        assert!(!contains(m, PhysicalPosition::new(0.0, 1080.0)));
    }

    #[test]
    fn cursor_on_secondary_monitor_left_of_primary() {
        // 副屏在主屏左侧（负坐标）：主屏判定不含，副屏判定含
        let primary = monitor(0, 0, 1920, 1080);
        let secondary = monitor(-1920, 0, 1920, 1080);
        let cursor = PhysicalPosition::new(-100.0, 500.0);
        assert!(!contains(primary, cursor));
        assert!(contains(secondary, cursor));
    }
}
