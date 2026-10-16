//! 浮层窗口模式：无边框、置顶、失焦即隐（hide 不销毁）。剪贴板浮层与命令
//! 面板定位到鼠标所在显示器居中，划词翻译定位到光标旁。
//! 被用户固定的浮层（固定浮层）不因失焦自动隐藏。

use std::collections::HashSet;
use std::sync::Mutex;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WindowEvent,
};

/// 光标旁定位时光标与浮层边缘的间距（物理像素）。
const CURSOR_MARGIN: f64 = 12.0;

/// 浮层逻辑尺寸（逻辑像素宽高）。
pub struct OverlaySize {
    pub width: f64,
    pub height: f64,
}

/// 被用户固定的浮层窗口 label 集合：固定浮层不因失焦自动隐藏。
#[derive(Default)]
pub struct PinnedOverlays(Mutex<HashSet<String>>);

impl PinnedOverlays {
    pub fn set(&self, label: &str, pinned: bool) {
        let mut labels = self.0.lock().unwrap();
        if pinned {
            labels.insert(label.to_string());
        } else {
            labels.remove(label);
        }
    }

    fn is_pinned(&self, label: &str) -> bool {
        self.0.lock().unwrap().contains(label)
    }
}

/// 固定/取消固定浮层。前端传自己的窗口 label；窗口不存在时报错。
#[tauri::command]
pub fn overlay_set_pinned(
    app: AppHandle,
    pinned_overlays: State<'_, PinnedOverlays>,
    label: String,
    pinned: bool,
) -> Result<(), String> {
    if app.get_webview_window(&label).is_none() {
        return Err(format!("浮层窗口不存在: {label}"));
    }
    pinned_overlays.set(&label, pinned);
    Ok(())
}

/// 创建一个隐藏的浮层窗口并挂上失焦即隐。`size` 为逻辑像素宽高。
/// 窗口以 hide 代替销毁，保证再次呼起瞬时。
///
/// 注意：WebView2 要求同一 user data folder 的所有 webview 使用完全一致的
/// browser args（实测：只给主窗口加 additionalBrowserArgs 会让隐藏的浮层
/// 创建报 0x8007139F INVALID_STATE）；新增浮层窗口保持默认参数即可。
pub fn create(
    app: &AppHandle,
    label: &str,
    path: &str,
    title: &str,
    size: OverlaySize,
) -> tauri::Result<()> {
    let window = tauri::WebviewWindowBuilder::new(app, label, WebviewUrl::App(path.into()))
        .title(title)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .inner_size(size.width, size.height)
        .build()?;

    let owner = app.clone();
    let hidden_label = label.to_string();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            // 固定浮层不因失焦隐藏；状态未初始化时按未固定处理
            let pinned = owner
                .try_state::<PinnedOverlays>()
                .is_some_and(|pinned| pinned.is_pinned(&hidden_label));
            if pinned {
                return;
            }
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
        window.set_focus()?;
        // 全局快捷键触发瞬间用户往往仍按着修饰键，不清掉的话浮层内按 ↓
        // 会带着 Alt 残留误开系统窗口菜单（清理原语与回帖/取词路径共用）
        if let Err(e) = crate::platform::release_lingering_modifiers() {
            eprintln!("youmi: 清理浮层残留修饰键失败: {e}");
        }
        Ok(())
    }
}

/// 呼起浮层并定位到光标旁（划词翻译用）：取词完成时鼠标就停在选中文本附近。
/// 定位失败降级为原位显示。
pub fn show_at_cursor(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(()); // 浮层初始化失败的降级场景
    };
    let moved = (|| -> tauri::Result<()> {
        let cursor = app.cursor_position()?;
        let Some(monitor) = find_monitor_containing(app, cursor)? else {
            return Ok(());
        };
        let size = window.outer_size()?;
        let position = near_cursor_position(
            cursor,
            (*monitor.position(), *monitor.size()),
            (f64::from(size.width), f64::from(size.height)),
            CURSOR_MARGIN,
        );
        window.set_position(position)
    })();
    if let Err(e) = moved {
        eprintln!("youmi: 浮层定位到光标旁失败: {e}");
    }
    window.show()?;
    window.set_focus()
}

/// 把窗口移到鼠标所在显示器居中；取不到光标/显示器时保持原位。
fn move_to_cursor_monitor(app: &AppHandle, window: &WebviewWindow) {
    let moved = (|| -> tauri::Result<()> {
        let cursor = app.cursor_position()?;
        let Some(monitor) = find_monitor_containing(app, cursor)? else {
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

fn find_monitor_containing(
    app: &AppHandle,
    cursor: PhysicalPosition<f64>,
) -> tauri::Result<Option<tauri::Monitor>> {
    Ok(app
        .available_monitors()?
        .into_iter()
        .find(|m| contains((*m.position(), *m.size()), cursor)))
}

/// 浮层左上角定位（物理坐标）：优先光标右下方 `margin` 处；右侧/下方放不下
/// 时翻到光标左侧/上方；最后 clamp 进显示器范围（浮层大于显示器时贴左/上）。
fn near_cursor_position(
    cursor: PhysicalPosition<f64>,
    monitor: (PhysicalPosition<i32>, PhysicalSize<u32>),
    size: (f64, f64),
    margin: f64,
) -> PhysicalPosition<i32> {
    let (position, bounds) = monitor;
    let left = f64::from(position.x);
    let top = f64::from(position.y);
    let right = left + f64::from(bounds.width);
    let bottom = top + f64::from(bounds.height);

    // 单轴定位：正向起点（光标 + margin）放得下就用，否则翻到反向起点
    // （光标 - margin - 跨度）；再 clamp 进 [low, high - span]（span > 轴长时贴 low）
    let place = |forward: f64, backward: f64, span: f64, low: f64, high: f64| -> i32 {
        let raw = if forward + span <= high {
            forward
        } else {
            backward
        };
        raw.clamp(low, (high - span).max(low)).round() as i32
    };
    let x = place(
        cursor.x + margin,
        cursor.x - margin - size.0,
        size.0,
        left,
        right,
    );
    let y = place(
        cursor.y + margin,
        cursor.y - margin - size.1,
        size.1,
        top,
        bottom,
    );
    PhysicalPosition::new(x, y)
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
    use super::{contains, near_cursor_position};
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

    fn pos(x: f64, y: f64) -> PhysicalPosition<f64> {
        PhysicalPosition::new(x, y)
    }

    #[test]
    fn cursor_inside_monitor_is_contained() {
        let m = monitor(0, 0, 1920, 1080);
        assert!(contains(m, pos(0.0, 0.0)));
        assert!(contains(m, pos(1919.5, 1079.5)));
    }

    #[test]
    fn cursor_on_right_or_bottom_edge_belongs_to_next_monitor() {
        let m = monitor(0, 0, 1920, 1080);
        assert!(!contains(m, pos(1920.0, 0.0)));
        assert!(!contains(m, pos(0.0, 1080.0)));
    }

    #[test]
    fn cursor_on_secondary_monitor_left_of_primary() {
        // 副屏在主屏左侧（负坐标）：主屏判定不含，副屏判定含
        let primary = monitor(0, 0, 1920, 1080);
        let secondary = monitor(-1920, 0, 1920, 1080);
        let cursor = pos(-100.0, 500.0);
        assert!(!contains(primary, cursor));
        assert!(contains(secondary, cursor));
    }

    const MARGIN: f64 = 12.0;

    #[test]
    fn overlay_prefers_below_right_of_cursor() {
        let m = monitor(0, 0, 1920, 1080);
        let position = near_cursor_position(pos(500.0, 400.0), m, (300.0, 200.0), MARGIN);
        assert_eq!(position, PhysicalPosition::new(512, 412));
    }

    #[test]
    fn overlay_flips_left_when_overflowing_right_edge() {
        let m = monitor(0, 0, 1920, 1080);
        // 光标右侧只剩 100：右侧放不下（100 < 300），翻到光标左侧
        let position = near_cursor_position(pos(1800.0, 400.0), m, (300.0, 200.0), MARGIN);
        assert_eq!(position, PhysicalPosition::new(1488, 412));
    }

    #[test]
    fn overlay_flips_up_when_overflowing_bottom_edge() {
        let m = monitor(0, 0, 1920, 1080);
        let position = near_cursor_position(pos(500.0, 1000.0), m, (300.0, 200.0), MARGIN);
        assert_eq!(position, PhysicalPosition::new(512, 788));
    }

    #[test]
    fn overlay_clamps_into_monitor() {
        let m = monitor(0, 0, 1920, 1080);
        // 光标在显示器右下角附近：翻转 + clamp 后浮层完整落在显示器内
        let position = near_cursor_position(pos(1919.0, 1079.0), m, (300.0, 200.0), MARGIN);
        assert_eq!(position, PhysicalPosition::new(1607, 867));
    }

    #[test]
    fn overlay_wider_than_monitor_pins_to_edge() {
        let m = monitor(0, 0, 1920, 1080);
        let position = near_cursor_position(pos(960.0, 540.0), m, (2000.0, 200.0), MARGIN);
        // 轴向放不下：clamp 到 (1920-2000) 与 0 的较大者 = 0
        assert_eq!(position, PhysicalPosition::new(0, 552));
    }

    #[test]
    fn overlay_position_accounts_for_monitor_offset() {
        // 副屏在主屏左侧（负坐标）：定位以副屏坐标为基准
        let m = monitor(-1920, 0, 1920, 1080);
        let position = near_cursor_position(pos(-1800.0, 400.0), m, (300.0, 200.0), MARGIN);
        assert_eq!(position, PhysicalPosition::new(-1788, 412));
    }
}
