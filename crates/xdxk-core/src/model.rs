//! ONNX 模型定位。
//!
//! CLI 从仓库根目录运行，模型就在 `models/common_lite.onnx`；图形界面运行时
//! 当前目录是构建产物目录（`build/windows/x64/runner/Debug`），所以要多找几处：
//! 环境变量 → 当前目录 → 可执行文件目录 → 逐级往上找。

use std::path::{Path, PathBuf};

/// 相对仓库根目录的模型路径。
pub const MODEL_RELATIVE: &str = "models/common_lite.onnx";

/// 环境变量：直接指定模型文件。
pub const MODEL_ENV: &str = "XDXK_MODEL";

fn push_unique(list: &mut Vec<PathBuf>, path: PathBuf) {
    if !list.contains(&path) {
        list.push(path);
    }
}

/// 按优先级列出所有候选路径。
pub fn candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    if let Ok(explicit) = std::env::var(MODEL_ENV) {
        if !explicit.trim().is_empty() {
            push_unique(&mut out, PathBuf::from(explicit));
        }
    }

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.to_path_buf());
        }
    }

    for root in &roots {
        push_unique(&mut out, root.join(MODEL_RELATIVE));
    }
    for root in &roots {
        out.extend(
            walk_up(root, 8)
                .into_iter()
                .map(|dir| dir.join(MODEL_RELATIVE)),
        );
    }
    out
}

fn walk_up(from: &Path, levels: usize) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut current = Some(from);
    for _ in 0..levels {
        let Some(dir) = current else { break };
        match dir.parent() {
            Some(parent) => {
                dirs.push(parent.to_path_buf());
                current = Some(parent);
            }
            None => break,
        }
    }
    dirs
}

/// 找到的第一个存在的模型文件。
pub fn resolve() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.is_file())
}

/// 给界面和日志用的一句话说明。
pub fn status_line() -> String {
    match resolve() {
        Some(path) => format!("模型：{}", path.display()),
        None => format!(
            "未找到 {MODEL_RELATIVE}（可用环境变量 {MODEL_ENV} 指定路径），自动识别验证码不可用"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_end_with_model_file() {
        let list = candidates();
        assert!(!list.is_empty());
        for path in list {
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some("common_lite.onnx"),
                "候选路径应当是模型文件: {}",
                path.display()
            );
        }
    }
}
