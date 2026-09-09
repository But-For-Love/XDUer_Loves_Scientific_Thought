mod captcha;
mod client;
mod config;
mod encrypt;

use std::collections::HashSet;

use clap::{Parser, Subcommand};
use config::Config;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::UtcOffset;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(version, about = "西电选课工具")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// 选课
    Select {
        /// 课程类别：0=必修 1=选修
        #[arg(short = 'c', long, default_value_t = 0)]
        category: u8,
        /// 仅尝试一次（不持续重试）
        #[arg(long)]
        once: bool,
    },
    /// 退课
    Drop {
        /// 课程类别：0=必修 1=选修
        #[arg(short = 'c', long, default_value_t = 0)]
        category: u8,
        /// 仅尝试一次（不持续重试）
        #[arg(long)]
        once: bool,
    },
    /// 容量检查 / 捡漏
    Check {
        /// 捡漏模式（扫描全部课程，否则只盯 conf 里配置的课程）
        #[arg(long)]
        snipe: bool,
    },
    /// 只读诊断课程是否存在及其类别/课序号
    Inspect,
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let cli = Cli::parse();

    let beijing = UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC);
    tracing_subscriber::fmt()
        .with_timer(OffsetTime::new(beijing, Rfc3339))
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,ort=error")),
        )
        .init();

    let mut conf = match Config::load("conf.json") {
        Ok(c) => c,
        Err(e) => {
            error!("{e}");
            return;
        }
    };

    // CLI 子命令覆盖配置（无子命令时按 conf.json 的 action 走）
    match &cli.command {
        Some(Command::Select { category, once }) => {
            conf.action = "add".to_string();
            conf.bx_or_xx = *category as i64;
            if *once {
                conf.always = 0;
            }
        }
        Some(Command::Drop { category, once }) => {
            conf.action = "del".to_string();
            conf.bx_or_xx = *category as i64;
            if *once {
                conf.always = 0;
            }
        }
        Some(Command::Check { snipe }) => {
            conf.action = if *snipe {
                "snipe".to_string()
            } else {
                "check".to_string()
            };
        }
        Some(Command::Inspect) => {}
        None => {}
    }

    if conf.always != 0 && conf.always != 1 {
        error!("always 应为 1（连续）或 0（仅一次）");
        return;
    }

    let check_mode = conf.action == "check" || conf.action == "snipe";
    let inspect_mode = matches!(cli.command, Some(Command::Inspect));

    let tasks = if check_mode || inspect_mode {
        Vec::new()
    } else {
        match config::build_tasks(&conf) {
            Ok(t) => t,
            Err(e) => {
                error!("{e}");
                return;
            }
        }
    };

    let client = match client::ApiClient::new() {
        Ok(client) => client,
        Err(e) => {
            error!(error = %e, "初始化 HTTP 客户端失败");
            return;
        }
    };

    let login_resp = match login_with_captcha(&client, &conf).await {
        Ok(r) => r,
        Err(e) => {
            error!("{e}");
            return;
        }
    };

    let token = match login_resp["data"]["token"].as_str() {
        Some(t) => t.to_string(),
        None => {
            error!(
                "登录失败：{}",
                login_resp["msg"].as_str().unwrap_or("未知错误")
            );
            return;
        }
    };

    let (batch, batch_name) = match client::pick_batch(&login_resp, &conf.batch) {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "选择批次失败");
            return;
        }
    };
    info!("选课批次：{batch_name}");

    let class_types = match client.enter_batch_page(&token, &batch).await {
        Ok(class_types) => class_types,
        Err(e) => {
            error!(error = %e, "进入选课页面失败");
            return;
        }
    };
    info!(?class_types, "当前批次课程类型");
    if let Err(e) = client.activate_batch(&token, &batch).await {
        error!(error = %e, "初始化选课批次失败");
        return;
    }

    if inspect_mode {
        inspect_courses(&client, &conf, &token, &batch, &class_types).await;
        return;
    }

    if check_mode {
        run_check_mode(&client, &conf, &token, &batch, &conf.action, &class_types).await;
        return;
    }

    let specific_types: Vec<&String> = class_types
        .iter()
        .filter(|class_type| class_type.as_str() != "ALLKC")
        .collect();
    let search_types: Vec<&String> = if specific_types.is_empty() {
        class_types.iter().collect()
    } else {
        specific_types
    };
    let mut listings = Vec::new();
    for class_type in search_types {
        match client.get_classes_by_type(&token, &batch, class_type).await {
            Ok(rows) => {
                listings.push((class_type.clone(), rows));
            }
            Err(e) => {
                error!("{e}");
                return;
            }
        }
    }

    let cancel = cancellation_on_ctrl_c();
    let mut scheduled = HashSet::new();
    for t in &tasks {
        let mut matched = false;
        'types: for (class_type, rows) in &listings {
            for row in rows {
                if value_text(&row["KCH"]).trim() != t.kch.trim() {
                    continue;
                }
                if let Some(tc_list) = row["tcList"].as_array() {
                    for class in tc_list {
                        if sequence_eq(&class["KXH"], &t.kxh) {
                            run_task_once(
                                &mut scheduled,
                                &client,
                                &token,
                                &batch,
                                &t.action,
                                class_type,
                                class,
                                conf.always,
                                &cancel,
                            )
                            .await;
                            matched = true;
                            break 'types;
                        }
                    }
                } else if sequence_eq(&row["KXH"], &t.kxh) {
                    run_task_once(
                        &mut scheduled,
                        &client,
                        &token,
                        &batch,
                        &t.action,
                        class_type,
                        row,
                        conf.always,
                        &cancel,
                    )
                    .await;
                    matched = true;
                    break 'types;
                }
            }
        }
        if !matched {
            let kind = if t.cat == 0 { "必修" } else { "选修" };
            let kcm = if t.kcm.is_empty() {
                String::new()
            } else {
                format!(" {}", t.kcm)
            };
            let kxh = format!(" {}", t.kxh);
            warn!("未找到课程（{kind}）：{}{}{}", t.kch, kcm, kxh);
        }
    }
}

async fn login_with_captcha(
    client: &client::ApiClient,
    conf: &Config,
) -> Result<Value, client::ClientError> {
    const MAX_OCR_ATTEMPTS: u8 = 3;
    let attempts = if conf.ocr_captcha == "1" {
        MAX_OCR_ATTEMPTS
    } else {
        1
    };
    for attempt in 1..=attempts {
        let (png, uuid) = client.fetch_captcha().await?;
        let code = if conf.ocr_captcha == "1" {
            match captcha::recognize(&png).await {
                Some(code) if code.chars().count() == 4 => {
                    info!(attempt, "验证码自动识别完成");
                    code
                }
                _ => {
                    warn!(attempt, "验证码自动识别结果无效，重新获取");
                    continue;
                }
            }
        } else {
            manual_input(&png)
        };
        match client.login(conf, &code, &uuid).await {
            Ok(response) => return Ok(response),
            Err(error) if error.is_captcha_error() && attempt < attempts => {
                warn!(attempt, "验证码错误，重新获取并识别");
            }
            Err(error) => return Err(error),
        }
    }
    Err(client::ClientError::InvalidResponse {
        operation: "登录",
        field: "验证码连续识别失败",
    })
}

async fn inspect_courses(
    client: &client::ApiClient,
    conf: &Config,
    token: &str,
    batch: &str,
    class_types: &[String],
) {
    let targets: HashSet<&str> = conf
        .bx
        .iter()
        .chain(&conf.xx)
        .map(|course| course.kch.trim())
        .filter(|course| !course.is_empty())
        .collect();

    let has_specific_types = class_types.iter().any(|class_type| class_type != "ALLKC");
    for class_type in class_types
        .iter()
        .filter(|class_type| !has_specific_types || class_type.as_str() != "ALLKC")
    {
        let rows = match client.get_classes_by_type(token, batch, class_type).await {
            Ok(rows) => rows,
            Err(e) => {
                error!(error = %e, class_type, "读取课程列表失败");
                if e.is_auth_expired() {
                    error!("会话已失效；请退出选课网站的其他登录后重新运行 inspect");
                    return;
                }
                continue;
            }
        };
        info!(class_type, rows = rows.len(), "只读课程诊断");
        for target in &targets {
            let matches: Vec<&Value> = rows
                .iter()
                .filter(|row| value_text(&row["KCH"]).trim() == *target)
                .collect();
            if matches.is_empty() {
                info!(class_type, kch = *target, "该类别没有目标课程");
                continue;
            }
            for row in matches {
                let sequences: Vec<String> = if let Some(tc_list) = row["tcList"].as_array() {
                    tc_list
                        .iter()
                        .map(|class| value_text(&class["KXH"]))
                        .filter(|value| !value.is_empty())
                        .collect()
                } else {
                    let kxh = value_text(&row["KXH"]);
                    if kxh.is_empty() {
                        Vec::new()
                    } else {
                        vec![kxh]
                    }
                };
                info!(
                    class_type,
                    kch = *target,
                    kcm = value_text(&row["KCM"]),
                    kxh = ?sequences,
                    "找到目标课程"
                );
            }
        }
    }
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    }
}

fn sequence_eq(value: &Value, configured: &str) -> bool {
    value_text(value).trim() == configured.trim()
}

/// 严格匹配：KCH 与 KXH 都按精确值比对（仅忽略首尾空白）；snipe（targets=None）时不筛选。
fn target_matches(targets: &Option<HashSet<(String, String)>>, kch: &str, kxh: &Value) -> bool {
    let Some(targets) = targets else {
        return true;
    };
    targets.iter().any(|(target_kch, target_kxh)| {
        target_kch.trim() == kch && sequence_eq(kxh, target_kxh.as_str())
    })
}

/// 容量检查 / 捡漏模式：轮询课程列表，见空位自动抢。
async fn run_check_mode(
    client: &client::ApiClient,
    conf: &Config,
    token: &str,
    batch: &str,
    mode: &str,
    class_types: &[String],
) {
    let targets: Option<HashSet<(String, String)>> = if mode == "check" {
        let mut s = HashSet::new();
        for c in &conf.bx {
            if !c.kch.is_empty() && !c.kxh.is_empty() {
                s.insert((c.kch.clone(), c.kxh.clone()));
            }
        }
        for c in &conf.xx {
            if !c.kch.is_empty() && !c.kxh.is_empty() {
                s.insert((c.kch.clone(), c.kxh.clone()));
            }
        }
        if s.is_empty() {
            error!("容量检查需要先在 conf.json 的 bx/xx 中配置课程号");
            return;
        }
        Some(s)
    } else {
        None
    };

    // 间隔可配置：interval_ms > 0 时使用，否则按模式默认（check=500ms, snipe=5s）
    let interval = if conf.interval_ms > 0 {
        std::time::Duration::from_millis(conf.interval_ms)
    } else if mode == "check" {
        std::time::Duration::from_millis(500)
    } else {
        std::time::Duration::from_secs(5)
    };

    let cancel = cancellation_on_ctrl_c();
    let mut completed = HashSet::new();

    let mut round = 0u32;
    loop {
        round += 1;
        let has_specific_types = class_types.iter().any(|class_type| class_type != "ALLKC");
        for class_type in class_types
            .iter()
            .filter(|class_type| !has_specific_types || class_type.as_str() != "ALLKC")
        {
            let rows = match client.get_classes_by_type(token, batch, class_type).await {
                Ok(rows) => rows,
                Err(e) => {
                    error!(error = %e, class_type, "扫描课程失败");
                    if e.is_auth_expired() {
                        error!("认证已失效，停止扫描");
                        return;
                    }
                    continue;
                }
            };
            for row in &rows {
                let kch = value_text(&row["KCH"]);
                let kch = kch.trim();
                if let Some(ref targets) = targets {
                    if !targets
                        .iter()
                        .any(|(target_kch, _)| target_kch.trim() == kch)
                    {
                        continue;
                    }
                }
                if let Some(tc_list) = row["tcList"].as_array() {
                    for tc in tc_list {
                        let kxh = value_text(&tc["KXH"]);
                        if !target_matches(&targets, kch, &tc["KXH"]) {
                            continue;
                        }
                        let clazz_id = tc["JXBID"].as_str().unwrap_or("");
                        if completed.contains(clazz_id) {
                            continue;
                        }
                        if has_capacity(tc) {
                            info!(
                                class_type,
                                kch,
                                kxh,
                                kcm = value_text(&row["KCM"]),
                                "发现空位"
                            );
                            match client
                                .add_course(token, batch, class_type, tc, false, &cancel)
                                .await
                            {
                                Ok(_) => {
                                    completed.insert(clazz_id.to_owned());
                                }
                                Err(e) => error!(error = %e, kch, kxh, "提交选课失败"),
                            }
                        }
                    }
                } else if target_matches(&targets, kch, &row["KXH"])
                    && !completed.contains(row["JXBID"].as_str().unwrap_or(""))
                    && has_capacity(row)
                {
                    let kxh = value_text(&row["KXH"]);
                    info!(
                        class_type,
                        kch,
                        kxh,
                        kcm = value_text(&row["KCM"]),
                        "发现空位"
                    );
                    let clazz_id = row["JXBID"].as_str().unwrap_or("");
                    match client
                        .add_course(token, batch, class_type, row, false, &cancel)
                        .await
                    {
                        Ok(_) => {
                            completed.insert(clazz_id.to_owned());
                        }
                        Err(e) => error!(error = %e, kch, kxh, "提交选课失败"),
                    }
                }
            }
        }
        info!("第 {round} 轮扫描完成");
        if cancel.is_cancelled() {
            info!("收到停止信号，退出容量检查/捡漏");
            break;
        }
        tokio::select! {
            () = tokio::time::sleep(interval) => {}
            () = cancel.cancelled() => break,
        }
    }
}

/// 判断课程是否还有空位（SFYX == "0" 且已选人数 < 容量）。
fn has_capacity(course: &Value) -> bool {
    let sfyx = course["SFYX"].as_str().unwrap_or("");
    let Some(sel) = as_u64(&course["numberOfSelected"]) else {
        return false;
    };
    let Some(cap) = as_u64(&course["classCapacity"]) else {
        return false;
    };
    sfyx == "0" && cap > 0 && sel < cap
}

/// 从 JSON 中安全取 u64（兼容数字或字符串）。
fn as_u64(v: &Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        Some(n)
    } else if let Some(s) = v.as_str() {
        s.parse().ok()
    } else {
        None
    }
}

fn manual_input(png: &[u8]) -> String {
    if let Err(e) = captcha::save_png(png, "captcha.png") {
        error!("保存验证码失败：{e}");
    }
    info!("验证码图片已保存为 captcha.png，请打开查看");
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut input) {
        error!(error = %e, "读取验证码输入失败");
        return String::new();
    }
    input.trim().to_string()
}

struct TaskRun<'a> {
    client: &'a client::ApiClient,
    token: &'a str,
    batch: &'a str,
    action: &'a str,
    class_type: &'a str,
    class: &'a Value,
    always: i64,
    cancel: &'a CancellationToken,
}

async fn run_task(task: TaskRun<'_>) -> Result<(), client::ClientError> {
    let kch = task.class["KCH"].as_str().unwrap_or("");
    let kcm = task.class["KCM"].as_str().unwrap_or("");
    match task.action {
        "query" => {
            info!(
                class_type = task.class_type,
                kch,
                kcm,
                kxh = value_text(&task.class["KXH"]),
                "匹配到课程"
            );
        }
        "del" => {
            task.client
                .delete_course(
                    task.token,
                    task.batch,
                    task.class_type,
                    task.class,
                    task.always == 1,
                    task.cancel,
                )
                .await?;
        }
        _ => {
            task.client
                .add_course(
                    task.token,
                    task.batch,
                    task.class_type,
                    task.class,
                    task.always == 1,
                    task.cancel,
                )
                .await?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_task_once(
    scheduled: &mut HashSet<String>,
    client: &client::ApiClient,
    token: &str,
    batch: &str,
    action: &str,
    class_type: &str,
    class: &Value,
    always: i64,
    cancel: &CancellationToken,
) {
    let Some(clazz_id) = class["JXBID"].as_str() else {
        error!("课程缺少 JXBID，跳过任务");
        return;
    };
    if !scheduled.insert(format!("{action}:{clazz_id}")) {
        warn!(clazz_id, action, "跳过重复课程任务");
        return;
    }
    if let Err(e) = run_task(TaskRun {
        client,
        token,
        batch,
        action,
        class_type,
        class,
        always,
        cancel,
    })
    .await
    {
        error!(error = %e, "课程任务失败");
    }
}

fn cancellation_on_ctrl_c() -> CancellationToken {
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.cancel();
        }
    });
    cancel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_matches_exactly() {
        assert!(sequence_eq(&Value::String("02".to_owned()), "02"));
        assert!(sequence_eq(&Value::from(2), "2"));
        assert!(!sequence_eq(&Value::String("2".to_owned()), "02"));
        assert!(!sequence_eq(&Value::String("02".to_owned()), "2"));
        assert!(!sequence_eq(&Value::String("88".to_owned()), "87"));
    }
}
