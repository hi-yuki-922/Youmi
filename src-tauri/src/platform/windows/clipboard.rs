//! clipboard-win 后端：剪贴板读写 + WM_CLIPBOARDUPDATE 监听（ADR-0001 clipboard-listener seam）。
//!
//! 平台实现不做单测：行为由 `#[ignore]` smoke 测试 + 手动验收覆盖。

use clipboard_win::formats;
use clipboard_win::{get_clipboard, get_clipboard_string, is_format_avail, set_clipboard_string};

use crate::clipboard::model::ClipboardContent;
use crate::platform::{ClipboardAccess, ClipboardWatcher, PlatformError};

use super::bmp;

pub struct WinClipboardAccess;

impl ClipboardAccess for WinClipboardAccess {
    fn read(&self) -> Option<ClipboardContent> {
        if is_format_avail(formats::CF_UNICODETEXT) {
            return get_clipboard_string().ok().map(ClipboardContent::Text);
        }
        if is_format_avail(formats::CF_DIB) {
            let bmp = get_clipboard(formats::Bitmap).ok()?;
            let png = bmp::bmp_to_png(&bmp).ok()?;
            return Some(ClipboardContent::ImagePng(png));
        }
        None
    }

    fn sequence(&self) -> Option<u64> {
        clipboard_win::seq_num().map(|n| u64::from(n.get()))
    }

    fn write_text(&self, text: &str) -> Result<(), PlatformError> {
        set_clipboard_string(text).map_err(|e| PlatformError(format!("写剪贴板文本失败: {e}")))
    }

    fn write_image_png(&self, png: &[u8]) -> Result<(), PlatformError> {
        let bmp = bmp::png_to_bmp(png).map_err(PlatformError)?;
        // CF_BITMAP 是 GDI 句柄格式，必须走 raw::set_bitmap（内部 CreateDIBitmap）；
        // 它为向后兼容不清空剪贴板（6.x 才改），故自行 open → empty → set。
        let mut result = Ok(());
        clipboard_win::with_clipboard(|| {
            result = clipboard_win::empty().and_then(|()| clipboard_win::raw::set_bitmap(&bmp));
        })
        .map_err(|e| PlatformError(format!("打开剪贴板失败: {e}")))?;
        result.map_err(|e| PlatformError(format!("写剪贴板图片失败: {e}")))
    }
}

pub struct WinClipboardWatcher;

impl ClipboardWatcher for WinClipboardWatcher {
    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Result<(), PlatformError> {
        // Monitor 依赖线程亲和的 message-only 窗口：必须在调用线程创建并在此 pump 消息。
        let mut monitor = clipboard_win::Monitor::new()
            .map_err(|e| PlatformError(format!("剪贴板监听初始化失败: {e}")))?;
        loop {
            match monitor.recv() {
                Ok(true) => on_change(),
                Ok(false) => return Ok(()), // shutdown 通道触发，正常退出
                Err(e) => return Err(PlatformError(format!("剪贴板监听中断: {e}"))),
            }
        }
    }
}

/// 真实剪贴板 smoke 测试：跑在实机上但会动系统剪贴板，默认忽略，
/// 用 `cargo test -- --ignored --test-threads=1` 手动执行。
#[cfg(test)]
mod smoke_tests {
    use super::*;
    use image::GenericImageView;

    fn access() -> WinClipboardAccess {
        WinClipboardAccess
    }

    #[test]
    #[ignore = "操作真实剪贴板，手动执行：cargo test -- --ignored"]
    fn text_write_read_roundtrip() {
        let access = access();
        access.write_text("youmi-smoke-文本").unwrap();
        assert_eq!(
            access.read(),
            Some(ClipboardContent::Text("youmi-smoke-文本".into()))
        );
    }

    #[test]
    #[ignore = "操作真实剪贴板，手动执行：cargo test -- --ignored"]
    fn image_write_read_roundtrip() {
        let access = access();
        let mut img = image::RgbaImage::new(4, 3);
        for (x, _, px) in img.enumerate_pixels_mut() {
            *px = image::Rgba([x as u8 * 60, 100, 200, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();

        access.write_image_png(&png).unwrap();
        let read_back = access.read().expect("剪贴板应可读出图片");
        let ClipboardContent::ImagePng(read_png) = read_back else {
            panic!("应读出图片，实际 {read_back:?}");
        };
        let decoded =
            image::load_from_memory_with_format(&read_png, image::ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (4, 3));
        // alpha 按不透明处理，RGB 无损
        assert_eq!(decoded.get_pixel(3, 0), image::Rgba([180, 100, 200, 255]));
    }

    #[test]
    #[ignore = "操作真实剪贴板，手动执行：cargo test -- --ignored"]
    fn watcher_receives_own_process_write_notification() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel::<()>();
        let watcher = WinClipboardWatcher;
        // watch 永不返回，线程随测试进程结束回收
        std::thread::spawn(move || {
            let _ = watcher.watch(Box::new(move || {
                let _ = tx.send(());
            }));
        });
        // 等 message-only 窗口与监听注册就绪
        std::thread::sleep(Duration::from_millis(300));

        set_clipboard_string("youmi-watcher-smoke").unwrap();

        let got = rx.recv_timeout(Duration::from_secs(3));
        assert!(got.is_ok(), "3 秒内未收到同进程写入的剪贴板变化通知");
    }
}
