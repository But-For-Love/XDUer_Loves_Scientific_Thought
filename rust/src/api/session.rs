//! 会话状态：登录、批次、课程列表、选课 / 退课。
//!
//! 界面每次只调一个函数，函数内部自己加锁，所以 Dart 侧不需要维护连接状态。
//! 登录后的 token 和 cookie 留在 Rust 进程里，不进 Dart。

use std::sync::OnceLock;

use flutter_rust_bridge::frb;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use xdxk_core::captcha;
use xdxk_core::client::{self, ApiClient};
use xdxk_core::config::Config;

use super::dto::{BatchView, CaptchaView, CourseRow, LoginView, SubmitResult, TaskView};

struct State {
    client: ApiClient,
    /// 当前验证码的 uuid（登录要用）
    uuid: String,
    png: Vec<u8>,
    token: Option<String>,
    batches: Vec<BatchView>,
    batch: Option<BatchView>,
    class_types: Vec<String>,
    loginname: String,
}

impl State {
    fn new() -> Self {
        Self {
            client: ApiClient::new().expect("构建 HTTP 客户端失败"),
            uuid: String::new(),
            png: Vec::new(),
            token: None,
            batches: Vec::new(),
            batch: None,
            class_types: Vec::new(),
            loginname: String::new(),
        }
    }

    fn token(&self) -> anyhow::Result<String> {
        self.token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("还没有登录"))
    }

    fn batch_code(&self) -> anyhow::Result<String> {
        self.batch
            .as_ref()
            .map(|batch| batch.code.clone())
            .ok_or_else(|| anyhow::anyhow!("还没有进入选课批次"))
    }
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::new()))
}

fn cancel_slot() -> &'static std::sync::Mutex<Option<CancellationToken>> {
    static SLOT: OnceLock<std::sync::Mutex<Option<CancellationToken>>> = OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

/// 停掉正在重试的选课 / 退课：重试循环会在下一次请求前退出。
#[frb(sync)]
pub fn stop() {
    if let Ok(slot) = cancel_slot().lock() {
        if let Some(token) = slot.as_ref() {
            token.cancel();
            warn!("收到停止请求，正在退出重试");
        }
    }
}

fn begin_cancel() -> CancellationToken {
    let token = CancellationToken::new();
    if let Ok(mut slot) = cancel_slot().lock() {
        *slot = Some(token.clone());
    }
    token
}

fn end_cancel() {
    if let Ok(mut slot) = cancel_slot().lock() {
        *slot = None;
    }
}

/// 取一张新验证码。图片是 PNG 字节，直接交给界面显示。
pub async fn fetch_captcha() -> anyhow::Result<CaptchaView> {
    let api = { state().lock().await.client.clone() };
    let (png, uuid) = api.fetch_captcha().await?;
    info!(bytes = png.len(), "已获取验证码");
    let mut guard = state().lock().await;
    guard.png = png.clone();
    guard.uuid = uuid.clone();
    Ok(CaptchaView {
        png,
        uuid,
        ocr_text: String::new(),
    })
}

/// 用本地模型识别手上这张验证码，把结果交给界面填进输入框。
pub async fn ocr_captcha() -> anyhow::Result<String> {
    let png = { state().lock().await.png.clone() };
    if png.is_empty() {
        anyhow::bail!("还没有验证码图片，先点「换一张」");
    }
    let text = captcha::recognize(&png)
        .await
        .ok_or_else(|| anyhow::anyhow!("识别不出结果。{}", xdxk_core::model::status_line()))?;
    if text.chars().count() != 4 {
        anyhow::bail!("识别结果「{text}」不是 4 位，请手动输入");
    }
    info!(text, "验证码识别完成");
    Ok(text)
}

/// 手动模式：用界面上那张验证码对应的字符登录。
pub async fn login(loginname: String, password: String, code: String) -> anyhow::Result<LoginView> {
    let (api, uuid) = {
        let guard = state().lock().await;
        (guard.client.clone(), guard.uuid.clone())
    };
    if uuid.is_empty() {
        anyhow::bail!("请先获取验证码");
    }
    let conf = Config::login_only(&loginname, &password);
    let response = api.login(&conf, &code, &uuid).await?;
    store_login(&conf, response).await
}

/// 自动模式：取码 → 识别 → 登录，最多三次，每次都在日志里留痕。
pub async fn login_auto(loginname: String, password: String) -> anyhow::Result<LoginView> {
    let api = { state().lock().await.client.clone() };
    let conf = Config::login_only(&loginname, &password);
    let mut last_error: Option<String> = None;
    for attempt in 1..=3u32 {
        let (png, uuid) = api.fetch_captcha().await?;
        {
            let mut guard = state().lock().await;
            guard.png = png.clone();
            guard.uuid = uuid.clone();
        }
        let Some(code) = captcha::recognize(&png).await else {
            last_error = Some("验证码识别没有结果".to_string());
            warn!(attempt, "验证码识别没有结果，重试");
            continue;
        };
        if code.chars().count() != 4 {
            last_error = Some(format!("识别结果「{code}」不是 4 位"));
            warn!(attempt, code, "识别结果不是 4 位，重试");
            continue;
        }
        info!(attempt, code, "识别到验证码，尝试登录");
        match api.login(&conf, &code, &uuid).await {
            Ok(response) => return store_login(&conf, response).await,
            Err(error) => {
                last_error = Some(error.to_string());
                warn!(attempt, %error, "这次登录没成功");
            }
        }
    }
    anyhow::bail!(
        "连续三次都没登上：{}。可以关掉自动识别，手动输验证码",
        last_error.unwrap_or_else(|| "未知原因".to_string())
    )
}

/// 登录成功后的统一收尾：把 token 和批次列表记到会话里。
async fn store_login(conf: &Config, response: Value) -> anyhow::Result<LoginView> {
    let token = response
        .pointer("/data/token")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "登录失败：{}",
                response["msg"].as_str().unwrap_or("没有拿到 token")
            )
        })?;
    let batches = parse_batches(&response);
    let view = LoginView {
        loginname: conf.data.loginname.clone(),
        message: response["msg"].as_str().unwrap_or("登录成功").to_string(),
        batches: batches.clone(),
    };
    {
        let mut guard = state().lock().await;
        guard.token = Some(token);
        guard.batches = batches.clone();
        guard.loginname = conf.data.loginname.clone();
    }
    info!(
        loginname = conf.data.loginname,
        batches = batches.len(),
        "登录成功"
    );
    Ok(view)
}

fn parse_batches(response: &Value) -> Vec<BatchView> {
    response
        .pointer("/data/student/electiveBatchList")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|item| BatchView {
                    code: client::value_text(&item["code"]),
                    name: client::value_text(&item["name"]),
                    open: client::value_text(&item["canSelect"]) == "1",
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 进入某个批次：拿课程类别（必修 / 选修 / 体育 ……）。
pub async fn open_batch(index: u32) -> anyhow::Result<Vec<String>> {
    let (api, token, batch) = {
        let guard = state().lock().await;
        let batch = guard
            .batches
            .get(index as usize)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("批次不存在，重新登录一次"))?;
        (guard.client.clone(), guard.token()?, batch)
    };
    let class_types = api.enter_batch_page(&token, &batch.code).await?;
    api.activate_batch(&token, &batch.code).await?;
    info!(batch = batch.name, ?class_types, "已进入选课批次");
    let mut guard = state().lock().await;
    guard.batch = Some(batch);
    guard.class_types = class_types.clone();
    Ok(class_types)
}

/// 读取某一类课程的全部教学班（一门课的每个课序号一行）。
pub async fn list_classes(class_type: String) -> anyhow::Result<Vec<CourseRow>> {
    let (api, token, batch) = {
        let guard = state().lock().await;
        (guard.client.clone(), guard.token()?, guard.batch_code()?)
    };
    let rows = api.get_classes_by_type(&token, &batch, &class_type).await?;
    let out = flatten_rows(&class_type, &rows);
    info!(class_type, rows = out.len(), "读取课程列表");
    Ok(out)
}

fn flatten_rows(class_type: &str, rows: &[Value]) -> Vec<CourseRow> {
    let mut out: Vec<CourseRow> = Vec::new();
    for row in rows {
        let kch = client::value_text(&row["KCH"]);
        let kcm = client::value_text(&row["KCM"]);
        match row["tcList"].as_array() {
            Some(list) if !list.is_empty() => {
                for node in list {
                    let no = out.len() as u32 + 1;
                    out.push(build_row(class_type, Some(row), node, &kch, &kcm, no));
                }
            }
            _ => {
                let no = out.len() as u32 + 1;
                out.push(build_row(class_type, None, row, &kch, &kcm, no));
            }
        }
    }
    out
}

/// 先看教学班这一层，再看课程那一层：同一个字段常常只有一层有。
fn pick(parent: Option<&Value>, node: &Value, keys: &[&str]) -> String {
    for key in keys {
        for source in [Some(node), parent].into_iter().flatten() {
            let text = client::value_text(&source[*key]);
            if !text.trim().is_empty() {
                return text;
            }
        }
    }
    String::new()
}

fn build_row(
    class_type: &str,
    parent: Option<&Value>,
    node: &Value,
    kch: &str,
    kcm: &str,
    no: u32,
) -> CourseRow {
    let capacity =
        client::as_u64(&Value::String(pick(parent, node, &["classCapacity"]))).unwrap_or(0) as i64;
    let selected = client::as_u64(&Value::String(pick(parent, node, &["numberOfSelected"])))
        .unwrap_or(0) as i64;
    let open = pick(parent, node, &["SFYX"]) == "0";
    let name = if kcm.is_empty() {
        pick(parent, node, &["KCM"])
    } else {
        kcm.to_string()
    };
    CourseRow {
        no,
        kch: kch.to_string(),
        kcm: name,
        kxh: pick(parent, node, &["KXH"]),
        class_type: class_type.to_string(),
        clazz_id: pick(parent, node, &["JXBID"]),
        secret: pick(parent, node, &["secretVal"]),
        teacher: pick(parent, node, &["SKJS", "SKJSXM", "JSXM", "teacher"]),
        time: pick(parent, node, &["SKSJ", "SKSM", "time"]),
        place: pick(parent, node, &["SKDD", "JASMC", "place"]),
        credit: pick(parent, node, &["XF", "credit"]),
        capacity,
        selected,
        available: open && capacity > 0 && selected < capacity,
        raw: node.to_string(),
    }
}

/// 选课 / 退课：对选中的行逐个提交，重试逻辑在 core 里。
pub async fn submit(
    rows: Vec<CourseRow>,
    action: String,
    continuous: bool,
) -> anyhow::Result<Vec<SubmitResult>> {
    let (api, token, batch) = {
        let guard = state().lock().await;
        (guard.client.clone(), guard.token()?, guard.batch_code()?)
    };
    if rows.is_empty() {
        anyhow::bail!("没有选中课程");
    }
    if action != "add" && action != "del" && action != "query" {
        anyhow::bail!("action 只能是 add / del / query");
    }
    let cancel = begin_cancel();
    let mut out = Vec::new();
    for row in rows {
        out.push(submit_one(&api, &token, &batch, &row, &action, continuous, &cancel).await);
    }
    end_cancel();
    let done = out.iter().filter(|item| item.ok).count();
    info!(done, total = out.len(), "本轮提交结束");
    Ok(out)
}

async fn submit_one(
    api: &ApiClient,
    token: &str,
    batch: &str,
    row: &CourseRow,
    action: &str,
    continuous: bool,
    cancel: &CancellationToken,
) -> SubmitResult {
    if cancel.is_cancelled() {
        return SubmitResult {
            kch: row.kch.clone(),
            kxh: row.kxh.clone(),
            action: action.to_string(),
            ok: false,
            message: "已停止".to_string(),
        };
    }
    let class = json!({
        "JXBID": row.clazz_id,
        "secretVal": row.secret,
        "KCH": row.kch,
        "KXH": row.kxh,
        "KCM": row.kcm,
    });
    let result = match action {
        "del" => {
            api.delete_course(token, batch, &row.class_type, &class, continuous, cancel)
                .await
        }
        "query" => {
            info!(
                kch = row.kch,
                kxh = row.kxh,
                "查询模式：只确认这条任务能匹配到课程"
            );
            Ok("已匹配".to_string())
        }
        _ => {
            api.add_course(token, batch, &row.class_type, &class, continuous, cancel)
                .await
        }
    };
    match result {
        Ok(message) => SubmitResult {
            kch: row.kch.clone(),
            kxh: row.kxh.clone(),
            action: action.to_string(),
            ok: true,
            message,
        },
        Err(error) => SubmitResult {
            kch: row.kch.clone(),
            kxh: row.kxh.clone(),
            action: action.to_string(),
            ok: false,
            message: error.to_string(),
        },
    }
}

/// 按任务列表跑一遍：先在当前批次各类课程里找到 KCH + KXH，再选 / 退。
/// 这是「一键开抢」用的入口，等价于 CLI 无子命令时的行为。
pub async fn run_tasks(
    tasks: Vec<TaskView>,
    continuous: bool,
) -> anyhow::Result<Vec<SubmitResult>> {
    let (api, token, batch, class_types) = {
        let guard = state().lock().await;
        (
            guard.client.clone(),
            guard.token()?,
            guard.batch_code()?,
            guard.class_types.clone(),
        )
    };
    if tasks.is_empty() {
        anyhow::bail!("任务列表是空的，先在「任务」里加几条");
    }
    let cancel = begin_cancel();
    let mut flat: Vec<CourseRow> = Vec::new();
    for class_type in &class_types {
        match api.get_classes_by_type(&token, &batch, class_type).await {
            Ok(rows) => flat.extend(flatten_rows(class_type, &rows)),
            Err(error) if error.is_auth_expired() => {
                end_cancel();
                anyhow::bail!("会话已失效，请重新登录");
            }
            Err(error) => warn!(class_type, %error, "读取课程列表失败，跳过这一类"),
        }
    }
    info!(
        classes = flat.len(),
        tasks = tasks.len(),
        "课程列表就绪，开始匹配任务"
    );

    let mut out = Vec::new();
    for task in tasks {
        let action = if task.action.is_empty() {
            "add".to_string()
        } else {
            task.action.clone()
        };
        let hit = flat
            .iter()
            .find(|row| row.kch.trim() == task.kch.trim() && row.kxh.trim() == task.kxh.trim());
        match hit {
            Some(row) => {
                out.push(submit_one(&api, &token, &batch, row, &action, continuous, &cancel).await)
            }
            None => {
                warn!(kch = task.kch, kxh = task.kxh, "课程列表里没有这条任务");
                out.push(SubmitResult {
                    kch: task.kch.clone(),
                    kxh: task.kxh.clone(),
                    action,
                    ok: false,
                    message: "当前批次的课程列表里没有这个课程号 + 课序号".to_string(),
                });
            }
        }
    }
    end_cancel();
    Ok(out)
}

/// 只读诊断：这些课程号在哪些类别里、有哪些课序号。不会选课。
pub async fn inspect(kchs: Vec<String>) -> anyhow::Result<Vec<String>> {
    let (api, token, batch, class_types) = {
        let guard = state().lock().await;
        (
            guard.client.clone(),
            guard.token()?,
            guard.batch_code()?,
            guard.class_types.clone(),
        )
    };
    let targets: Vec<String> = kchs
        .into_iter()
        .map(|kch| kch.trim().to_string())
        .filter(|kch| !kch.is_empty())
        .collect();
    if targets.is_empty() {
        anyhow::bail!("先填课程号（KCH）");
    }
    let mut lines = Vec::new();
    for class_type in class_types {
        let rows = match api.get_classes_by_type(&token, &batch, &class_type).await {
            Ok(rows) => rows,
            Err(error) if error.is_auth_expired() => {
                anyhow::bail!("会话已失效，请重新登录");
            }
            Err(error) => {
                lines.push(format!("{class_type}：读取失败（{error}）"));
                continue;
            }
        };
        for target in &targets {
            let hits: Vec<&Value> = rows
                .iter()
                .filter(|row| client::value_text(&row["KCH"]).trim() == target)
                .collect();
            for row in hits {
                let sequences: Vec<String> = match row["tcList"].as_array() {
                    Some(list) => list
                        .iter()
                        .map(|tc| client::value_text(&tc["KXH"]))
                        .filter(|kxh| !kxh.is_empty())
                        .collect(),
                    None => vec![client::value_text(&row["KXH"])],
                };
                let line = format!(
                    "{class_type} │ {} │ {} │ 课序号 {}",
                    target,
                    client::value_text(&row["KCM"]),
                    sequences.join(" / ")
                );
                info!("{line}");
                lines.push(line);
            }
        }
    }
    if lines.is_empty() {
        lines.push("这些课程号在当前批次里都没找到，核对 KCH 的大小写和前导零".to_string());
    }
    Ok(lines)
}
