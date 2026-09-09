use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{header::COOKIE, Client, Response};
use serde_json::{json, Value};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::config::Config;
use crate::encrypt::aes_encrypt;

const CAPTCHA_URL: &str = "https://xk.xidian.edu.cn/xsxk/auth/captcha";
const LOGIN_URL: &str = "https://xk.xidian.edu.cn/xsxk/auth/login";
const CLASS_LIST_URL: &str = "https://xk.xidian.edu.cn/xsxk/elective/clazz/list";
const ELECTIVE_USER_URL: &str = "https://xk.xidian.edu.cn/xsxk/elective/user";
const GRAB_LESSONS_URL: &str = "https://xk.xidian.edu.cn/xsxk/elective/grablessons";
const ADD_URL: &str = "https://xk.xidian.edu.cn/xsxk/elective/clazz/add";
const DEL_URL: &str = "https://xk.xidian.edu.cn/xsxk/elective/clazz/del";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44";

const ADD_STOP_MSGS: [&str; 5] = [
    "该课程已在选课结果中",
    "所选课程与已选课程冲突",
    "所选课程人数已满",
    "操作成功",
    "选课门数或学分超过",
];
const DROP_STOP_MSGS: [&str; 2] = ["所选课程与已选课程冲突", "操作成功"];

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("构建 HTTP client 失败: {0}")]
    Build(#[source] reqwest::Error),
    #[error("{operation}请求失败: {source}")]
    Request {
        operation: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{operation}响应解析失败: {source}")]
    Decode {
        operation: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{operation}响应缺少字段: {field}")]
    InvalidResponse {
        operation: &'static str,
        field: &'static str,
    },
    #[error("{operation}响应内容解码失败: {message}")]
    Content {
        operation: &'static str,
        message: String,
    },
    #[error("{operation}业务失败: code={code}, msg={message}")]
    Business {
        operation: &'static str,
        code: String,
        message: String,
    },
    #[error("操作已取消")]
    Cancelled,
}

#[derive(Clone)]
pub struct ApiClient {
    http: Client,
}

struct OperationRequest<'a> {
    url: &'static str,
    form: Vec<(&'static str, &'a str)>,
    token: &'a str,
    batch: &'a str,
    course_id: &'a str,
    course_sequence: &'a str,
    course_name: &'a str,
    operation: &'static str,
    continuous: bool,
    is_stop: fn(&str) -> bool,
    retry_sleep: Option<Duration>,
}

impl ApiClient {
    pub fn new() -> Result<Self, ClientError> {
        let http = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(ClientError::Build)?;
        Ok(Self { http })
    }

    pub async fn fetch_captcha(&self) -> Result<(Vec<u8>, String), ClientError> {
        let response =
            self.http
                .post(CAPTCHA_URL)
                .send()
                .await
                .map_err(|source| ClientError::Request {
                    operation: "获取验证码",
                    source,
                })?;
        let response = successful_response(response, "获取验证码")?;
        let value: Value = response
            .json()
            .await
            .map_err(|source| ClientError::Decode {
                operation: "获取验证码",
                source,
            })?;
        ensure_success(&value, "获取验证码")?;
        let data_uri = required_str(&value, &["data", "captcha"], "获取验证码", "data.captcha")?;
        let pic = data_uri
            .strip_prefix("data:image/png;base64,")
            .unwrap_or(data_uri);
        let bytes = STANDARD.decode(pic).map_err(|error| ClientError::Content {
            operation: "获取验证码",
            message: format!("验证码 base64 解码失败: {error}"),
        })?;
        let uuid = required_str(&value, &["data", "uuid"], "获取验证码", "data.uuid")?;
        Ok((bytes, uuid.to_owned()))
    }

    pub async fn login(&self, conf: &Config, code: &str, uuid: &str) -> Result<Value, ClientError> {
        let encrypted_password = aes_encrypt(&conf.data.password);
        let form = [
            ("loginname", conf.data.loginname.as_str()),
            ("password", encrypted_password.as_str()),
            ("captcha", code),
            ("uuid", uuid),
        ];
        let response = self
            .http
            .post(LOGIN_URL)
            .header("Connection", "keep-alive")
            .header("User-Agent", USER_AGENT)
            .form(&form)
            .send()
            .await
            .map_err(|source| ClientError::Request {
                operation: "登录",
                source,
            })?;
        let response = successful_response(response, "登录")?;
        let value = response
            .json()
            .await
            .map_err(|source| ClientError::Decode {
                operation: "登录",
                source,
            })?;
        ensure_success(&value, "登录")?;
        Ok(value)
    }

    pub async fn get_classes_by_type(
        &self,
        token: &str,
        batch: &str,
        class_type: &str,
    ) -> Result<Vec<Value>, ClientError> {
        let mut body = json!({
            "teachingClassType": class_type,
            "pageNumber": 1,
            "pageSize": 300,
            "orderBy": "",
        });
        if class_type != "ALLKC" {
            body["campus"] = Value::String("S".to_owned());
        }
        let response = self
            .http
            .post(CLASS_LIST_URL)
            .header("Connection", "keep-alive")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("batchId", batch)
            .header("Authorization", token)
            .header(COOKIE, format!("Authorization={token}"))
            .header("User-Agent", USER_AGENT)
            .json(&body)
            .send()
            .await
            .map_err(|source| ClientError::Request {
                operation: "获取课程列表",
                source,
            })?;
        let response = successful_response(response, "获取课程列表")?;
        let value: Value = response
            .json()
            .await
            .map_err(|source| ClientError::Decode {
                operation: "获取课程列表",
                source,
            })?;
        ensure_success(&value, "获取课程列表")?;
        value["data"]["rows"]
            .as_array()
            .cloned()
            .ok_or(ClientError::InvalidResponse {
                operation: "获取课程列表",
                field: "data.rows",
            })
    }

    pub async fn activate_batch(&self, token: &str, batch: &str) -> Result<(), ClientError> {
        let response = self
            .http
            .post(ELECTIVE_USER_URL)
            .header("Authorization", token)
            .header(COOKIE, format!("Authorization={token}"))
            .form(&[("batchId", batch)])
            .send()
            .await
            .map_err(|source| ClientError::Request {
                operation: "初始化选课批次",
                source,
            })?;
        let response = successful_response(response, "初始化选课批次")?;
        let value: Value = response
            .json()
            .await
            .map_err(|source| ClientError::Decode {
                operation: "初始化选课批次",
                source,
            })?;
        ensure_success(&value, "初始化选课批次")
    }

    pub async fn enter_batch_page(
        &self,
        token: &str,
        batch: &str,
    ) -> Result<Vec<String>, ClientError> {
        let response = self
            .http
            .get(GRAB_LESSONS_URL)
            .header("Authorization", token)
            .header(COOKIE, format!("Authorization={token}"))
            .query(&[("batchId", batch)])
            .send()
            .await
            .map_err(|source| ClientError::Request {
                operation: "进入选课页面",
                source,
            })?;
        let response = successful_response(response, "进入选课页面")?;
        info!(url = %response.url(), "已进入选课页面");
        let html = response
            .text()
            .await
            .map_err(|source| ClientError::Decode {
                operation: "读取选课页面",
                source,
            })?;
        parse_menu_types(&html)
    }

    pub async fn add_course(
        &self,
        token: &str,
        batch: &str,
        class_type: &str,
        class: &Value,
        continuous: bool,
        cancel: &CancellationToken,
    ) -> Result<String, ClientError> {
        let clazz_id = class["JXBID"].as_str().unwrap_or("");
        let secret = class["secretVal"].as_str().unwrap_or("");
        self.poll_operation(
            OperationRequest {
                url: ADD_URL,
                form: vec![
                    ("clazzType", class_type),
                    ("clazzId", clazz_id),
                    ("secretVal", secret),
                    ("chooseVolunteer", "1"),
                ],
                token,
                batch,
                course_id: class["KCH"].as_str().unwrap_or(""),
                course_sequence: class["KXH"].as_str().unwrap_or(""),
                course_name: class["KCM"].as_str().unwrap_or(""),
                operation: "选课",
                continuous,
                is_stop: is_add_stop,
                retry_sleep: Some(Duration::from_secs(1)),
            },
            cancel,
        )
        .await
    }

    pub async fn delete_course(
        &self,
        token: &str,
        batch: &str,
        class_type: &str,
        class: &Value,
        continuous: bool,
        cancel: &CancellationToken,
    ) -> Result<String, ClientError> {
        let clazz_id = class["JXBID"].as_str().unwrap_or("");
        let secret = class["secretVal"].as_str().unwrap_or("");
        let mut form = vec![
            ("clazzType", class_type),
            ("clazzId", clazz_id),
            ("secretVal", secret),
        ];
        if class_type == "XGKC" {
            form.push(("chooseVolunteer", "1"));
        }
        self.poll_operation(
            OperationRequest {
                url: DEL_URL,
                form,
                token,
                batch,
                course_id: class["KCH"].as_str().unwrap_or(""),
                course_sequence: class["KXH"].as_str().unwrap_or(""),
                course_name: class["KCM"].as_str().unwrap_or(""),
                operation: "退课",
                continuous,
                is_stop: is_drop_stop,
                retry_sleep: None,
            },
            cancel,
        )
        .await
    }

    async fn poll_operation(
        &self,
        request: OperationRequest<'_>,
        cancel: &CancellationToken,
    ) -> Result<String, ClientError> {
        let mut attempt = 0u32;
        loop {
            if cancel.is_cancelled() {
                return Err(ClientError::Cancelled);
            }
            attempt += 1;
            let started = Instant::now();
            let response = self
                .http
                .post(request.url)
                .header("User-Agent", USER_AGENT)
                .header("batchId", request.batch)
                .header("Authorization", request.token)
                .query(&request.form)
                .send()
                .await
                .map_err(|source| ClientError::Request {
                    operation: request.operation,
                    source,
                })?;
            let response = successful_response(response, request.operation)?;
            let value: Value = response
                .json()
                .await
                .map_err(|source| ClientError::Decode {
                    operation: request.operation,
                    source,
                })?;
            let message = value["msg"].as_str().unwrap_or("").to_owned();
            info!(
                operation = request.operation,
                attempt,
                course_id = request.course_id,
                kxh = request.course_sequence,
                course_name = request.course_name,
                latency_ms = started.elapsed().as_millis(),
                message,
                "课程操作完成"
            );
            if !request.continuous || (request.is_stop)(&message) {
                return Ok(message);
            }
            if let Some(duration) = request.retry_sleep {
                tokio::select! {
                    () = tokio::time::sleep(duration) => {}
                    () = cancel.cancelled() => return Err(ClientError::Cancelled),
                }
            }
        }
    }
}

impl ClientError {
    pub fn is_auth_expired(&self) -> bool {
        matches!(self, Self::Business { code, .. } if code == "401")
    }

    pub fn is_captcha_error(&self) -> bool {
        matches!(self, Self::Business { message, .. } if message.contains("验证码"))
    }
}

pub fn pick_batch(login_resp: &Value, batch: &str) -> Result<(String, String), ClientError> {
    let list = login_resp
        .pointer("/data/student/electiveBatchList")
        .and_then(Value::as_array)
        .ok_or(ClientError::InvalidResponse {
            operation: "选择批次",
            field: "data.student.electiveBatchList",
        })?;
    let mut first_available = None;
    let mut matched = None;
    for item in list {
        let can_select = item.get("canSelect").and_then(Value::as_str).unwrap_or("");
        let name = item.get("name").and_then(Value::as_str).unwrap_or("");
        let code = item.get("code").and_then(Value::as_str).unwrap_or("");
        if can_select == "1" {
            if first_available.is_none() {
                first_available = Some((code.to_owned(), name.to_owned()));
            }
            if !batch.is_empty() && name.contains(batch) && matched.is_none() {
                matched = Some((code.to_owned(), name.to_owned()));
            }
        }
    }
    if batch.is_empty() {
        first_available
    } else {
        matched
    }
    .ok_or(ClientError::InvalidResponse {
        operation: "选择批次",
        field: "未找到匹配且开放的批次",
    })
}

fn is_add_stop(message: &str) -> bool {
    ADD_STOP_MSGS.iter().any(|text| message.contains(text))
}

fn is_drop_stop(message: &str) -> bool {
    DROP_STOP_MSGS.iter().any(|text| message.contains(text))
}

fn required_str<'a>(
    value: &'a Value,
    path: &[&str],
    operation: &'static str,
    field: &'static str,
) -> Result<&'a str, ClientError> {
    path.iter()
        .try_fold(value, |current, key| current.get(key))
        .and_then(Value::as_str)
        .ok_or(ClientError::InvalidResponse { operation, field })
}

fn ensure_success(value: &Value, operation: &'static str) -> Result<(), ClientError> {
    let Some(code) = value.get("code") else {
        return Err(ClientError::InvalidResponse {
            operation,
            field: "code",
        });
    };
    if code.as_i64() == Some(200) || code.as_str() == Some("200") {
        return Ok(());
    }
    Err(ClientError::Business {
        operation,
        code: code
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| code.to_string()),
        message: value["msg"].as_str().unwrap_or("未知错误").to_owned(),
    })
}

fn successful_response(
    response: Response,
    operation: &'static str,
) -> Result<Response, ClientError> {
    response
        .error_for_status()
        .map_err(|source| ClientError::Request { operation, source })
}

fn parse_menu_types(html: &str) -> Result<Vec<String>, ClientError> {
    const PREFIX: &str = "grablessonsVue.menuData.menuList = ";
    let json = html
        .lines()
        .find_map(|line| line.trim().strip_prefix(PREFIX))
        .and_then(|value| value.strip_suffix(';'))
        .ok_or(ClientError::InvalidResponse {
            operation: "读取选课页面",
            field: "menuData.menuList",
        })?;
    let menu: Vec<Value> = serde_json::from_str(json).map_err(|error| ClientError::Content {
        operation: "读取选课页面",
        message: format!("解析课程类型失败: {error}"),
    })?;
    Ok(menu
        .iter()
        .filter_map(|item| item["teachingClassType"].as_str())
        .filter(|class_type| *class_type != "YXKC")
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_business_error_instead_of_empty_rows() {
        let value = json!({"code": 401, "msg": "请重新登录"});
        assert!(matches!(
            ensure_success(&value, "获取课程列表"),
            Err(ClientError::Business { code, .. }) if code == "401"
        ));
    }

    #[test]
    fn configured_batch_does_not_fall_back() {
        let value = json!({
            "data": {"student": {"electiveBatchList": [
                {"canSelect": "1", "name": "第一批", "code": "A"}
            ]}}
        });
        assert!(pick_batch(&value, "第二批").is_err());
    }

    #[test]
    fn parses_course_types_from_batch_page() {
        let html = r#"
            grablessonsVue.menuData.menuList = [{"teachingClassType":"TJKC"},{"teachingClassType":"TYKC"},{"teachingClassType":"YXKC"}];
        "#;
        assert_eq!(
            parse_menu_types(html).unwrap(),
            vec!["TJKC".to_owned(), "TYKC".to_owned()]
        );
    }
}
