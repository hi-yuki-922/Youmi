//! 测试专用支撑（仅 test 构建编译）。

use std::path::{Path, PathBuf};

/// 唯一临时目录（pid + 纳秒时间戳），drop 时尽力删除。
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "youmi-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // 尽力而为：连接未关闭等情况下 Windows 删除会失败，测试环境可容忍
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
