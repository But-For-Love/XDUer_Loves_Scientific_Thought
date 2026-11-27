//! 配置文件放在哪：一个明确的位置 + 可以由用户改、改完记住，不做路径猜测。
//!
//! 解析顺序：
//! 1. 环境变量 `XDXK_CONF`（命令行 / 自动化用，优先级最高，界面上不能改）
//! 2. `settings.json` 里记住的上次选择
//! 3. 默认：**exe 同目录的 `conf.json`**（绿色便携）；exe 目录不可写时回退到
//!    `%APPDATA%\xdxk-gui\conf.json`（其他平台 `~/.config/xdxk-gui/`）
//!
//! 想整个搬到别处（U 盘、便携目录），设 `XDXK_HOME` 指到那个目录即可。

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use anyhow::anyhow;
use flutter_rust_bridge::frb;
use tracing::info;

const CONFIG_NAME: &str = "conf.json";
const SETTINGS_NAME: &str = "settings.json";
const ENV_CONF: &str = "XDXK_CONF";
const ENV_HOME: &str = "XDXK_HOME";

/// exe 所在目录：发布版把配置就放这儿，拷走整个文件夹配置跟着走。
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

/// 目录能不能写：exe 装在 Program Files 里时不能写，要回退到用户目录。
fn dir_writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(".xdxk-write-test");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 用户级目录：`%APPDATA%\xdxk-gui`（其他平台 `~/.config/xdxk-gui`）。
fn user_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata = appdata.trim();
        if !appdata.is_empty() {
            return PathBuf::from(appdata).join("xdxk-gui");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("xdxk-gui");
    }
    PathBuf::from(".").join(".xdxk-gui")
}

/// 配置和设置的落脚目录：`XDXK_HOME` → exe 同目录（可写）→ 用户目录。
pub(crate) fn base_dir() -> PathBuf {
    if let Ok(value) = std::env::var(ENV_HOME) {
        let value = value.trim();
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    if let Some(dir) = exe_dir() {
        if dir_writable(&dir) {
            return dir;
        }
    }
    user_dir()
}

fn settings_file() -> PathBuf {
    base_dir().join(SETTINGS_NAME)
}

/// 默认位置：exe 旁边的 conf.json（不可写时用用户目录）。
fn default_path() -> PathBuf {
    base_dir().join(CONFIG_NAME)
}

/// 上次选过的配置文件路径；没选过就是空串。
fn saved_conf_path() -> String {
    let Ok(text) = std::fs::read_to_string(settings_file()) else {
        return String::new();
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|doc| {
            doc.get("conf_path")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

fn write_settings(conf_path: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(base_dir())?;
    let doc = serde_json::json!({ "conf_path": conf_path });
    std::fs::write(settings_file(), serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

fn initial_path() -> PathBuf {
    if let Ok(explicit) = std::env::var(ENV_CONF) {
        if !explicit.trim().is_empty() {
            return PathBuf::from(explicit.trim());
        }
    }
    let saved = saved_conf_path();
    if !saved.trim().is_empty() {
        return PathBuf::from(saved.trim());
    }
    default_path()
}

fn current() -> &'static RwLock<PathBuf> {
    static CURRENT: OnceLock<RwLock<PathBuf>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new(initial_path()))
}

/// 当前生效的配置文件路径（crate 内部用）。
pub(crate) fn current_conf_path() -> PathBuf {
    current()
        .read()
        .map(|path| path.clone())
        .unwrap_or_else(|_| default_path())
}

fn env_locked() -> bool {
    std::env::var(ENV_CONF)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

/// 当前生效的配置文件路径。
#[frb(sync)]
pub fn conf_path() -> String {
    current_conf_path().display().to_string()
}

/// 默认位置（恢复默认时用）：exe 旁边的 conf.json。
#[frb(sync)]
pub fn default_conf_path() -> String {
    default_path().display().to_string()
}

/// 设置文件所在目录（换过的路径记在这儿的 settings.json 里）。
#[frb(sync)]
pub fn app_data_dir() -> String {
    base_dir().display().to_string()
}

/// 路径是否被环境变量 `XDXK_CONF` 固定住了（固定后界面不能改）。
#[frb(sync)]
pub fn conf_locked() -> bool {
    env_locked()
}

/// 换一个配置文件，并记住。传目录就自动补 `conf.json`。
#[frb(sync)]
pub fn set_conf_path(path: String) -> anyhow::Result<String> {
    if env_locked() {
        anyhow::bail!("当前路径由环境变量 {ENV_CONF} 指定，先在系统里取消它再改");
    }
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("路径不能为空");
    }
    let mut target = PathBuf::from(trimmed);
    if target.is_dir() {
        target = target.join(CONFIG_NAME);
    }
    if !target.is_absolute() {
        anyhow::bail!("请给绝对路径：{trimmed}");
    }
    let parent = target
        .parent()
        .ok_or_else(|| anyhow!("这个路径没有上级目录：{}", target.display()))?;
    std::fs::create_dir_all(parent)?;

    *current().write().map_err(|_| anyhow!("状态锁坏了"))? = target.clone();
    write_settings(&target.display().to_string())?;
    info!(path = %target.display(), exists = target.is_file(), "已切换配置文件");
    Ok(target.display().to_string())
}

/// 回到默认位置（exe 旁边那份）。
#[frb(sync)]
pub fn reset_conf_path() -> anyhow::Result<String> {
    if env_locked() {
        anyhow::bail!("当前路径由环境变量 {ENV_CONF} 指定，先在系统里取消它再改");
    }
    let target = default_path();
    std::fs::create_dir_all(base_dir())?;
    *current().write().map_err(|_| anyhow!("状态锁坏了"))? = target.clone();
    write_settings("")?;
    info!(path = %target.display(), "配置路径已恢复默认");
    Ok(target.display().to_string())
}

/// 在资源管理器里定位当前配置文件（不存在就打开它所在的目录）。
#[frb(sync)]
pub fn reveal_conf() -> anyhow::Result<()> {
    let path = current_conf_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("explorer.exe");
        if path.is_file() {
            command.arg(format!("/select,{}", path.display()));
        } else {
            command.arg(path.parent().unwrap_or(Path::new(".")));
        }
        command.spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!(
            "当前平台没有实现「打开所在目录」，路径是 {}",
            path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home() -> PathBuf {
        std::env::temp_dir().join(format!("xdxk-settings-{}", std::process::id()))
    }

    #[test]
    fn switch_creates_settings_and_accepts_directory() {
        let home = temp_home();
        std::env::set_var(ENV_HOME, &home);

        let target = home.join("nested").join("my-conf.json");
        let actual = set_conf_path(target.display().to_string()).expect("设置路径");
        assert_eq!(PathBuf::from(&actual), target);
        assert_eq!(PathBuf::from(conf_path()), target);
        assert!(settings_file().is_file(), "settings.json 应该被写出来");
        assert_eq!(saved_conf_path(), target.display().to_string());

        // 传目录时补 conf.json
        let dir = home.join("dir");
        std::fs::create_dir_all(&dir).expect("建目录");
        let actual = set_conf_path(dir.display().to_string()).expect("设置目录");
        assert!(actual.ends_with("conf.json"), "实际路径 {actual}");

        // 恢复默认 = 这个目录下的 conf.json
        let default = reset_conf_path().expect("恢复默认");
        assert_eq!(default, default_conf_path());
        assert_eq!(saved_conf_path(), "");

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn relative_path_is_rejected() {
        assert!(set_conf_path("conf.json".to_string()).is_err());
        assert!(set_conf_path("   ".to_string()).is_err());
    }
}
