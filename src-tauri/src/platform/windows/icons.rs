//! 开始菜单快捷方式图标提取：SHGetFileInfoW 解析 .lnk 取目标图标句柄，
//! GetIconInfo + GetDIBits 读出 32bpp 像素，换 RGBA 序后编码为 PNG。

use std::io::Cursor;

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

/// 提取快捷方式（或可执行文件）的图标为 PNG 字节；失败返回 None（调用方回退通用图标）。
pub fn extract_icon_png(path: &str) -> Option<Vec<u8>> {
    let mut wide: Vec<u16> = path.encode_utf16().collect();
    wide.push(0);

    // .lnk 图标解析走 shell link handler，依赖 COM：与 launch_lnk 同款 STA
    // 初始化，成功的初始化（含 S_FALSE）用 CoUninitialize 配平
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let png = match icon_of_wide(&wide) {
        Some(hicon) => {
            let png = icon_to_png(hicon);
            let _ = unsafe { DestroyIcon(hicon) };
            png
        }
        None => None,
    };
    if com.is_ok() {
        unsafe { CoUninitialize() };
    }
    png
}

/// SHGetFileInfoW 解析路径取图标句柄；解析失败或句柄无效返回 None。
fn icon_of_wide(wide: &[u16]) -> Option<HICON> {
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    (ok != 0 && !info.hIcon.is_invalid()).then_some(info.hIcon)
}

/// 图标句柄 → 颜色位图 → PNG。单色图标（无颜色位图）按失败处理，交调用方回退。
fn icon_to_png(hicon: HICON) -> Option<Vec<u8>> {
    let mut info: ICONINFO = unsafe { std::mem::zeroed() };
    unsafe { GetIconInfo(hicon, &mut info) }.ok()?;
    if info.hbmColor.is_invalid() {
        let _ = unsafe { DeleteObject(HGDIOBJ(info.hbmMask.0)) };
        return None;
    }

    let hdc = unsafe { GetDC(None) };
    let png = bitmap_to_png(hdc, info.hbmColor);
    let _ = unsafe {
        ReleaseDC(None, hdc);
        let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
        DeleteObject(HGDIOBJ(info.hbmMask.0))
    };
    png
}

/// 颜色位图 → PNG：GetObjectW 取尺寸，单次 GetDIBits 按 top-down（负高度）
/// 读 32bpp，BGRA 换序为 RGBA 后交给 image 编码。异常尺寸按失败防御。
/// （实测 windows crate 下 GetDIBits 空缓冲查询恒返回 0，不能用它拿尺寸。）
fn bitmap_to_png(hdc: HDC, hbm: HBITMAP) -> Option<Vec<u8>> {
    let mut bmp = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            HGDIOBJ(hbm.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut BITMAP as *mut std::ffi::c_void),
        )
    };
    if got == 0 {
        return None;
    }
    let (width, height) = (bmp.bmWidth.unsigned_abs(), bmp.bmHeight.unsigned_abs());
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }

    let mut info = BITMAPINFO::default();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width as i32;
    info.bmiHeader.biHeight = -(height as i32); // 负高度 = top-down
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB.0;
    let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
    let read = unsafe {
        GetDIBits(
            hdc,
            hbm,
            0,
            height,
            Some(bgra.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if read != height as i32 {
        return None;
    }
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2); // BGRA → RGBA
    }

    let img = image::RgbaImage::from_raw(width, height, bgra)?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

#[cfg(test)]
mod smoke_tests {
    use super::extract_icon_png;

    /// 真实提取 smoke：从本机开始菜单抓一批真实 .lnk 提取图标，断言至少一个
    /// 产出合法 PNG（个别快捷方式无图标属正常，逐个打印便于诊断）。
    /// 依赖机器上存在开始菜单快捷方式，手动执行。
    #[test]
    #[ignore = "依赖真实开始菜单，手动执行：cargo test extract_icon_png -- --ignored"]
    fn extract_icon_png_produces_valid_png_from_real_lnk() {
        let root = std::env::var_os("APPDATA")
            .map(|appdata| {
                std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs")
            })
            .expect("环境缺少 APPDATA");

        let mut tried = 0;
        let mut extracted = 0;
        for entry in std::fs::read_dir(&root)
            .expect("开始菜单目录不可读")
            .flatten()
        {
            let path = entry.path();
            if !path.extension().is_some_and(|ext| ext == "lnk") {
                continue;
            }
            tried += 1;
            match extract_icon_png(path.to_string_lossy().as_ref()) {
                Some(png) => {
                    let decoded =
                        image::load_from_memory_with_format(&png, image::ImageFormat::Png)
                            .expect("提取结果不是合法 PNG");
                    assert!(decoded.width() > 0 && decoded.height() > 0);
                    extracted += 1;
                }
                None => eprintln!("无图标（回退通用）: {}", path.display()),
            }
            if tried >= 20 {
                break;
            }
        }
        assert!(tried > 0, "开始菜单根目录没有 .lnk，换一个目录跑");
        assert!(extracted > 0, "{tried} 个快捷方式全部提取失败");
    }
}
