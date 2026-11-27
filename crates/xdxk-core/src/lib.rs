//! 西电选课工具的核心逻辑。
//!
//! 这一层不关心界面：CLI（`crates/xdxk-cli`）和图形界面（`rust/` 里的
//! flutter_rust_bridge 层）用的是同一套代码，行为改一次两边都变。
//!
//! - [`config`]：`conf.json` 的读取与任务归一化
//! - [`client`]：教务系统 HTTP 接口封装
//! - [`captcha`]：验证码获取与 ONNX 识别
//! - [`encrypt`]：登录密码的 AES 加密
//! - [`model`]：ONNX 模型的路径解析

pub mod captcha;
pub mod client;
pub mod config;
pub mod encrypt;
pub mod model;
