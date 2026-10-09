//! 剪贴板历史域模型。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 历史条目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardKind {
    Text,
    Image,
}

impl ClipboardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ClipboardKind::Text => "text",
            ClipboardKind::Image => "image",
        }
    }
}

/// 数据库里 `kind` 列的反向解析。
impl std::str::FromStr for ClipboardKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(ClipboardKind::Text),
            "image" => Ok(ClipboardKind::Image),
            other => Err(format!("未知的剪贴板条目类型: {other}")),
        }
    }
}

/// 一次剪贴板内容的载体：平台 seam 读取的结果，也是自家写入通道的写入载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardContent {
    Text(String),
    /// 已编码为 PNG 的图片字节。
    ImagePng(Vec<u8>),
}

impl ClipboardContent {
    pub fn kind(&self) -> ClipboardKind {
        match self {
            ClipboardContent::Text(_) => ClipboardKind::Text,
            ClipboardContent::ImagePng(_) => ClipboardKind::Image,
        }
    }

    /// 内容指纹（SHA-256，带类型前缀避免跨类型碰撞）。
    /// 去重、自家写入排除都以它为键。
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            ClipboardContent::Text(text) => {
                hasher.update(b"text\0");
                hasher.update(text.as_bytes());
            }
            ClipboardContent::ImagePng(bytes) => {
                hasher.update(b"image\0");
                hasher.update(bytes);
            }
        }
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_has_same_fingerprint() {
        let a = ClipboardContent::Text("hello".into());
        let b = ClipboardContent::Text("hello".into());
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn different_content_has_different_fingerprint() {
        let text = ClipboardContent::Text("hello".into());
        let other_text = ClipboardContent::Text("world".into());
        let image = ClipboardContent::ImagePng(vec![1, 2, 3]);
        assert_ne!(text.fingerprint(), other_text.fingerprint());
        assert_ne!(text.fingerprint(), image.fingerprint());
    }

    #[test]
    fn kind_serializes_lowercase() {
        assert_eq!(ClipboardKind::Text.as_str(), "text");
        assert_eq!(ClipboardKind::Image.as_str(), "image");
        assert_eq!(
            serde_json::to_string(&ClipboardKind::Image).unwrap(),
            r#""image""#
        );
    }
}
