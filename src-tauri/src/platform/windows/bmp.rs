//! BMP 与 PNG 互转。
//!
//! clipboard-win 的 `Bitmap` 格式读写字节 = 带BITMAPFILEHEADER 的 BMP 文件。
//! 图片入库统一为 PNG（spec：图片以 PNG blob 存储），回写剪贴板再转回 BMP。
//!
//! 注意：32bpp BI_RGB 的第 4 字节按 Windows 语义是保留位（未定义），解码端按
//! 不透明处理——因此经剪贴板 roundtrip 的图片不带 alpha，RGB 像素无损。

use std::io::Cursor;

use image::ImageFormat;

/// 把剪贴板读出的 BMP 字节编码为 PNG。
pub fn bmp_to_png(bmp: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory_with_format(bmp, ImageFormat::Bmp)
        .map_err(|e| format!("BMP 解码失败: {e}"))?;
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
    Ok(png)
}

/// 把 PNG 字节编码为 32bpp BI_RGB bottom-up BMP（clipboard-win `Bitmap` Setter 的字节布局）。
pub fn png_to_bmp(png: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory_with_format(png, ImageFormat::Png)
        .map_err(|e| format!("PNG 解码失败: {e}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);

    let pixel_size = width * 4 * height; // 32bpp 行宽天然 4 字节对齐
    let data_offset: u32 = 14 + 40;
    let mut bmp = Vec::with_capacity(data_offset as usize + pixel_size);

    // BITMAPFILEHEADER
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((data_offset + pixel_size as u32).to_le_bytes()));
    bmp.extend_from_slice(&0u32.to_le_bytes()); // 保留字段
    bmp.extend_from_slice(&data_offset.to_le_bytes());

    // BITMAPINFOHEADER：正高度 = bottom-up
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(width as i32).to_le_bytes());
    bmp.extend_from_slice(&(height as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    bmp.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    bmp.extend_from_slice(&(pixel_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 16]); // 分辨率与调色板字段全 0

    for y in (0..height).rev() {
        for x in 0..width {
            let px = rgba.get_pixel(x as u32, y as u32);
            bmp.extend_from_slice(&[px[2], px[1], px[0], px[3]]); // RGBA → BGRA
        }
    }
    Ok(bmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn sample_png() -> Vec<u8> {
        let mut img = RgbaImage::new(3, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        img.put_pixel(2, 0, Rgba([0, 0, 255, 255]));
        img.put_pixel(0, 1, Rgba([255, 255, 0, 128])); // 半透明：roundtrip 后变不透明
        img.put_pixel(1, 1, Rgba([0, 255, 255, 255]));
        img.put_pixel(2, 1, Rgba([255, 255, 255, 255]));
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        png
    }

    fn decode_png(bytes: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .unwrap()
            .to_rgba8()
    }

    #[test]
    fn png_bmp_roundtrip_preserves_pixels_opaque_alpha() {
        let original = decode_png(&sample_png());

        let bmp = png_to_bmp(&sample_png()).unwrap();
        let roundtripped = decode_png(&bmp_to_png(&bmp).unwrap());

        assert_eq!(roundtripped.dimensions(), (3, 2));
        for (x, y, px) in original.enumerate_pixels() {
            let expected_alpha = 255u8; // BI_RGB 的 X 通道一律按不透明
            let got = roundtripped.get_pixel(x, y);
            assert_eq!(
                [got[0], got[1], got[2]],
                [px[0], px[1], px[2]],
                "pixel ({x},{y})"
            );
            assert_eq!(got[3], expected_alpha);
        }
    }

    #[test]
    fn png_to_bmp_writes_valid_headers() {
        let bmp = png_to_bmp(&sample_png()).unwrap();
        assert_eq!(&bmp[..2], b"BM");
        let file_size = u32::from_le_bytes(bmp[2..6].try_into().unwrap());
        assert_eq!(file_size as usize, bmp.len());
        let data_offset = u32::from_le_bytes(bmp[10..14].try_into().unwrap());
        assert_eq!(data_offset, 54);
        let width = i32::from_le_bytes(bmp[18..22].try_into().unwrap());
        let height = i32::from_le_bytes(bmp[22..26].try_into().unwrap());
        assert_eq!((width, height), (3, 2)); // 正高度 = bottom-up
        let bpp = u16::from_le_bytes(bmp[28..30].try_into().unwrap());
        assert_eq!(bpp, 32);
    }

    #[test]
    fn invalid_input_errors_cleanly() {
        assert!(png_to_bmp(b"not a png").is_err());
        assert!(bmp_to_png(b"not a bmp").is_err());
    }
}
