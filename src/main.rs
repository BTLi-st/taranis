mod price;
mod conf;

use chrono::NaiveDateTime;
use price::calc_price;
use tracing::{event, instrument};
use tracing_subscriber::{EnvFilter, Layer};
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::fmt::time::ChronoLocal;

#[instrument]
fn main() {
    // 打开日志文件
    let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 设置日志过滤器
    let console_filter = EnvFilter::new("info");
    let file_filter = EnvFilter::new("trace");

    // 设置控制台日志格式
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_ansi(true)
        .with_level(true)
        .with_target(false)
        .with_thread_names(true)
        .with_filter(console_filter);

    // 设置文件日志格式
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE | FmtSpan::NEW)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(file_filter);

    // 初始化日志订阅者
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    let t1 = NaiveDateTime::parse_from_str("2023-10-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let t2 = NaiveDateTime::parse_from_str("2023-10-02 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let power = 1.0;
    let price = calc_price(t1, t2, power).unwrap();
    event!(
        tracing::Level::INFO,
        "Calculated price: {}",
        price
    );
}