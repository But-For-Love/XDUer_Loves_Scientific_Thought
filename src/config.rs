use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ocr_captcha: String,
    #[serde(default)]
    #[allow(dead_code)] // TODO: 实现 debug 输出文件
    pub debug: String,
    #[serde(default)]
    pub batch: String,
    #[serde(default)]
    pub action: String,
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
    #[serde(default = "default_always")]
    pub always: i64,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default = "default_bx_or_xx")]
    pub bx_or_xx: i64,
    #[serde(default)]
    pub bx: Vec<Course>,
    #[serde(default)]
    pub xx: Vec<Course>,
    #[serde(default)]
    pub data: LoginData,
}

fn default_always() -> i64 {
    1
}
fn default_bx_or_xx() -> i64 {
    0
}
fn default_interval_ms() -> u64 {
    0
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Task {
    #[serde(rename = "type", default)]
    pub category: Value,
    #[serde(default)]
    pub action: String,
    #[serde(rename = "KCH", default)]
    pub kch: String,
    #[serde(rename = "KXH", default)]
    pub kxh: String,
    #[serde(rename = "KCM", default)]
    pub kcm: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Course {
    #[serde(rename = "KCH", default)]
    pub kch: String,
    #[serde(rename = "KXH", default)]
    pub kxh: String,
    #[serde(rename = "KCM", default)]
    pub kcm: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoginData {
    #[serde(default)]
    pub loginname: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    #[allow(dead_code)] // 占位字段，登录时会被重新获取覆盖
    pub captcha: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub uuid: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("读取配置失败: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("解析配置失败: {e}"))
    }
}

/// 归一化后的任务
#[derive(Debug, Clone)]
pub struct NormalizedTask {
    pub cat: u8,
    pub action: String,
    pub kch: String,
    pub kxh: String,
    pub kcm: String,
}

fn parse_category(v: &Value) -> Option<u8> {
    match v {
        Value::String(s) if s == "bx" => Some(0),
        Value::String(s) if s == "xx" => Some(1),
        Value::Number(n) if n.as_i64() == Some(0) => Some(0),
        Value::Number(n) if n.as_i64() == Some(1) => Some(1),
        _ => None,
    }
}

/// 复刻 Python 的 _build_tasks：任务列表优先，否则用旧的单类别配置。
pub fn build_tasks(conf: &Config) -> Result<Vec<NormalizedTask>, String> {
    if !conf.tasks.is_empty() {
        let mut out = Vec::new();
        for t in &conf.tasks {
            let cat = parse_category(&t.category)
                .ok_or_else(|| format!("tasks 项 type 不合法: {:?}", t.category))?;
            let action = if t.action.is_empty() {
                "add".to_string()
            } else {
                t.action.clone()
            };
            if action != "add" && action != "del" && action != "query" {
                return Err(format!("tasks 项 action 不合法: {action}"));
            }
            if t.kch.is_empty() || t.kxh.is_empty() {
                return Err(format!("tasks 项缺少 KCH 或 KXH: {:?}", t));
            }
            out.push(NormalizedTask {
                cat,
                action,
                kch: t.kch.clone(),
                kxh: t.kxh.clone(),
                kcm: t.kcm.clone(),
            });
        }
        return Ok(out);
    }

    let cat_i64 = conf.bx_or_xx;
    if cat_i64 != 0 && cat_i64 != 1 {
        return Err("bx_or_xx 应为 0（必修）或 1（选修）".to_string());
    }
    let cat: u8 = cat_i64 as u8;
    let action = if conf.action.is_empty() {
        "add".to_string()
    } else {
        conf.action.clone()
    };
    if action != "add" && action != "del" && action != "query" {
        return Err(format!("action 仅支持 add / del / query: {action}"));
    }
    let courses: &Vec<Course> = if cat == 0 { &conf.bx } else { &conf.xx };
    if courses.is_empty() {
        return Err("课程列表为空，请在 conf.json 中填写 bx 或 xx".to_string());
    }
    let mut out = Vec::new();
    for c in courses {
        if c.kch.is_empty() || c.kxh.is_empty() {
            return Err(format!("课程必填 KCH 和 KXH: {:?}", c));
        }
        out.push(NormalizedTask {
            cat,
            action: action.clone(),
            kch: c.kch.clone(),
            kxh: c.kxh.clone(),
            kcm: c.kcm.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_required() {
        let conf: Config = serde_json::from_str(
            r#"{"bx_or_xx": 0, "action": "add", "bx": [{"KCH": "TE", "KXH": "01"}]}"#,
        )
        .unwrap();
        let tasks = build_tasks(&conf).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].cat, 0);
        assert_eq!(tasks[0].kch, "TE");
        assert_eq!(tasks[0].kxh, "01");
    }

    #[test]
    fn task_list() {
        let conf: Config = serde_json::from_str(
            r#"{"tasks": [{"type": "bx", "action": "add", "KCH": "TE", "KXH": "01"},
                         {"type": "xx", "action": "del", "KCH": "FL", "KXH": "02"}]}"#,
        )
        .unwrap();
        let tasks = build_tasks(&conf).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].cat, 1);
        assert_eq!(tasks[1].action, "del");
    }
}
