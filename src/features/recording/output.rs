//! 【录制文件】【事务输出】独占工作目录，成功后发布成品，失败保留可恢复的视频

use std::path::{Path, PathBuf};

pub struct Output {
    pub directory: PathBuf,
    pub video: PathBuf,
    pub destination: PathBuf,
}

impl Output {
    /// 【录制文件】【路径预留】按 config 创建唯一工作目录并测试写入，返回路径或文件错误
    pub fn create(config: &crate::config::recording::Recording) -> Result<Self, String> {
        let root = if config.directory.trim().is_empty() {
            dirs::video_dir()
                .or_else(dirs::home_dir)
                .ok_or("无法定位视频目录 / Video directory unavailable")?
                .join("Mark Shot")
        } else {
            std::path::absolute(&config.directory).map_err(|error| error.to_string())?
        };
        std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
        let time = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        for suffix in 0..10000 {
            let name = format!("Recording_{time}_{suffix:03}");
            let destination = root.join(format!("{name}.{}", config.format));
            if destination.exists() {
                continue;
            }
            let directory = root.join(format!(".{name}.recording"));
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    let video = directory.join("capture.mp4");
                    return Ok(Self {
                        directory,
                        video,
                        destination,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("无法创建唯一录制文件 / Cannot reserve a recording file".into())
    }

    /// 【录制文件】【发布成品】按 source 发布非空成品，拒绝覆盖已有文件；成功后清理本次工作目录
    pub fn publish(&self, source: &Path) -> Result<(), String> {
        if std::fs::metadata(source)
            .map_err(|error| error.to_string())?
            .len()
            == 0
        {
            return Err("录制没有生成有效数据 / Recording produced no data".into());
        }
        // 1. 【录制文件】【独占发布】同文件系统硬链接原子创建成品，目标已存在时保持原文件
        std::fs::hard_link(source, &self.destination)
            .map_err(|error| format!("保存失败 / Publish failed: {error}"))?;
        std::fs::remove_dir_all(&self.directory).map_err(|error| {
            format!("成品已保存，临时文件清理失败 / Saved, cleanup failed: {error}")
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【录制测试】【成品保护】无参数，验证已有同名文件不被覆盖，工作视频仍可恢复
    #[test]
    fn publication_preserves_existing_destination() {
        let root = tempfile::tempdir().unwrap();
        let config = crate::config::recording::Recording {
            directory: root.path().to_string_lossy().into(),
            ..Default::default()
        };
        let output = Output::create(&config).unwrap();
        std::fs::write(&output.video, b"new video").unwrap();
        std::fs::write(&output.destination, b"existing").unwrap();
        assert!(output.publish(&output.video).is_err());
        assert_eq!(std::fs::read(&output.destination).unwrap(), b"existing");
        assert!(output.video.exists());
    }
}
