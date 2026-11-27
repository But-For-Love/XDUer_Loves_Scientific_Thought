//! Flutter 能调到的全部 Rust 函数都在这个模块树里。
//!
//! - [`dto`]：纯数据类型
//! - [`session`]：登录、批次、课程、选课 / 退课
//! - [`logs`]：日志流
//! - 本文件：启动、配置文件、环境信息

pub mod dto;
pub mod logs;
pub mod session;
pub mod settings;

use flutter_rust_bridge::frb;
use serde_json::{json, Value};
use tracing::{info, warn};
use xdxk_core::config::Config;

use dto::{ConfView, TaskView};

/// Flutter 侧 `await RustLib.init()` 时调用：装好错误处理与日志管道。
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    logs::init_tracing();
    info!("界面启动");
    info!("{}", xdxk_core::model::status_line());
    info!("配置文件：{}", settings::conf_path());
    info!("默认位置：{}", settings::default_conf_path());
}

/// 读 `conf.json`。文件不存在就用默认值，并在 `note` 里说一声。
#[frb(sync)]
pub fn load_config() -> ConfView {
    let path = settings::current_conf_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            warn!(%error, path = %path.display(), "读取配置失败，改用默认值");
            return ConfView {
                path: path.display().to_string(),
                exists: false,
                ocr_captcha: true,
                always: true,
                note: format!(
                    "还没有配置文件；填好账号和任务后点「保存配置」，会写到 {}",
                    path.display()
                ),
                ..Default::default()
            };
        }
    };
    match serde_json::from_str::<Config>(&text) {
        Ok(conf) => {
            info!(path = %path.display(), "已读取配置");
            let mut view = to_view(&conf);
            view.path = path.display().to_string();
            view.exists = true;
            view
        }
        Err(error) => {
            warn!(%error, "配置解析失败");
            ConfView {
                path: path.display().to_string(),
                exists: true,
                ocr_captcha: true,
                always: true,
                note: format!("conf.json 解析失败：{error}"),
                ..Default::default()
            }
        }
    }
}

fn to_view(conf: &Config) -> ConfView {
    let mut tasks: Vec<TaskView> = conf
        .tasks
        .iter()
        .map(|task| TaskView {
            category: match &task.category {
                Value::String(text) if text == "xx" => 1,
                Value::Number(number) if number.as_i64() == Some(1) => 1,
                _ => 0,
            },
            action: if task.action.is_empty() {
                "add".to_string()
            } else {
                task.action.clone()
            },
            kch: task.kch.clone(),
            kxh: task.kxh.clone(),
            kcm: task.kcm.clone(),
        })
        .collect();

    // 老配置没有 tasks：用 bx / xx + action 折算成任务列表
    if tasks.is_empty() {
        let category = if conf.bx_or_xx == 1 { 1u8 } else { 0u8 };
        let courses = if category == 0 { &conf.bx } else { &conf.xx };
        tasks = courses
            .iter()
            .map(|course| TaskView {
                category,
                action: if conf.action.is_empty() {
                    "add".to_string()
                } else {
                    conf.action.clone()
                },
                kch: course.kch.clone(),
                kxh: course.kxh.clone(),
                kcm: course.kcm.clone(),
            })
            .collect();
    }

    ConfView {
        path: String::new(),
        exists: false,
        loginname: conf.data.loginname.clone(),
        password: conf.data.password.clone(),
        batch: conf.batch.clone(),
        ocr_captcha: conf.ocr_captcha != "0",
        always: conf.always != 0,
        interval_ms: conf.interval_ms,
        tasks,
        note: String::new(),
    }
}

/// 写回当前配置文件。字段按 CLI 的格式写，界面版和命令行版可以共用同一份。
#[frb(sync)]
pub fn save_config(view: ConfView) -> anyhow::Result<String> {
    // 路径由 settings 决定（用户选过就用选的，否则默认位置），这里不接受覆盖，
    // 免得界面上「保存」和「当前配置」不是同一个文件。
    let path = settings::current_conf_path();
    let tasks: Vec<Value> = view
        .tasks
        .iter()
        .map(|task| {
            json!({
                "type": if task.category == 1 { "xx" } else { "bx" },
                "action": if task.action.is_empty() { "add" } else { task.action.as_str() },
                "KCH": task.kch,
                "KXH": task.kxh,
                "KCM": task.kcm,
            })
        })
        .collect();
    let first_action = view
        .tasks
        .first()
        .map(|task| task.action.clone())
        .filter(|action| !action.is_empty())
        .unwrap_or_else(|| "add".to_string());
    let first_category = view.tasks.first().map(|task| task.category).unwrap_or(0);

    let document = json!({
        "ocr_captcha": if view.ocr_captcha { "1" } else { "0" },
        "debug": "0",
        "batch": view.batch,
        "action": first_action,
        "interval_ms": view.interval_ms,
        "always": if view.always { 1 } else { 0 },
        "tasks": tasks,
        "bx_or_xx": first_category,
        "bx": [],
        "xx": [],
        "data": {
            "loginname": view.loginname,
            "password": view.password,
            "captcha": "",
            "uuid": "",
        },
    });

    let text = serde_json::to_string_pretty(&document)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(&path, text) {
        anyhow::bail!(
            "写不进 {}（{error}）。如果程序装在 Program Files，去「概览 → 配置文件 → 恢复默认」，会改存到用户目录",
            path.display()
        );
    }
    info!(path = %path.display(), tasks = view.tasks.len(), "配置已保存");
    Ok(path.display().to_string())
}

/// ONNX 模型在哪、能不能自动识别验证码。
#[frb(sync)]
pub fn model_status() -> String {
    xdxk_core::model::status_line()
}
