//! 日志管道：Rust 里所有 `tracing` 输出都会出现在界面的「日志」页。
//!
//! `tracing` 事件 → 全局 broadcast → `StreamSink` → Dart。界面断了也不影响程序，
//! 只是这一条丢掉。

use std::fmt;

use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// 屏幕上一行日志。
#[derive(Clone, Debug)]
pub struct LogLine {
    /// 北京时间 hh:mm:ss.mmm
    pub time: String,
    /// ERROR / WARN / INFO / DEBUG / TRACE
    pub level: String,
    /// 产生日志的模块，例如 xdxk_core::client
    pub target: String,
    pub message: String,
}

static LOG_TX: std::sync::OnceLock<broadcast::Sender<LogLine>> = std::sync::OnceLock::new();

fn sender() -> &'static broadcast::Sender<LogLine> {
    LOG_TX.get_or_init(|| broadcast::channel(2048).0)
}

pub(crate) fn subscribe() -> broadcast::Receiver<LogLine> {
    sender().subscribe()
}

pub(crate) fn emit(line: LogLine) {
    // 没有界面在听的时候 send 会失败，忽略即可。
    let _ = sender().send(line);
}

/// 北京时间的时间戳。
fn stamp() -> String {
    use time::format_description::FormatItem;
    use time::macros::format_description;
    use time::{OffsetDateTime, UtcOffset};
    const FMT: &[FormatItem<'static>] =
        format_description!("[hour]:[minute]:[second].[subsecond digits:3]");
    let offset = UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC);
    OffsetDateTime::now_utc()
        .to_offset(offset)
        .format(FMT)
        .unwrap_or_default()
}

struct FieldVisitor {
    message: String,
    fields: Vec<String>,
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }
}

struct ChannelLayer {}

impl<S> Layer<S> for ChannelLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor {
            message: String::new(),
            fields: Vec::new(),
        };
        event.record(&mut visitor);
        let meta = event.metadata();
        let message = match visitor.fields.len() {
            0 => visitor.message,
            _ => format!("{} · {}", visitor.message, visitor.fields.join(" ")),
        };
        emit(LogLine {
            time: stamp(),
            level: meta.level().to_string(),
            target: meta.target().to_string(),
            message,
        });
    }
}

/// 只初始化一次；界面上看到的就是 Rust 侧看到的。
pub(crate) fn init_tracing() {
    // 同时打到 stderr：用 `flutter run` 或控制台启动时能直接看到，界面里也有同样的一份。
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_writer(std::io::stderr);
    let _ = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,ort=error")),
        )
        .with(ChannelLayer {})
        .with(stderr_layer)
        .try_init();
}

/// 订阅日志流。Flutter 侧 `await for` 它。
#[frb]
pub fn log_stream(sink: StreamSink<LogLine>) -> anyhow::Result<()> {
    let mut receiver = subscribe();
    std::thread::spawn(move || loop {
        match receiver.blocking_recv() {
            Ok(line) => {
                if sink.add(line).is_err() {
                    break; // 界面已经把流关掉了
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                let _ = sink.add(LogLine {
                    time: stamp(),
                    level: "WARN".to_string(),
                    target: "bridge".to_string(),
                    message: format!("日志太快，丢弃了 {skipped} 条"),
                });
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    });
    Ok(())
}
