use kernel::config::server_config;
use std::env;
#[cfg(target_os = "windows")]
use time::format_description::well_known::Rfc3339;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::{Compact, Format};
#[cfg(target_os = "windows")]
use tracing_subscriber::fmt::time::LocalTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, fmt};

pub struct Logger {
    _file_guard: WorkerGuard,
    _stdout_guard: WorkerGuard,
}

impl Logger {
    pub fn init() -> anyhow::Result<Self> {
        // 获取配置
        let config = server_config();
        if env::var_os("RUST_LOG").is_none() {
            unsafe {
                env::set_var("RUST_LOG", &config.log_level.as_str());
            }
        }

        // 系统变量设置
        let log_env = match config.log_level.as_str() {
            "TRACE" => Level::TRACE,
            "DEBUG" => Level::DEBUG,
            "INFO" => Level::INFO,
            "WARN" => Level::WARN,
            "ERROR" => Level::ERROR,
            _ => Level::INFO,
        };

        let format = get_log_format();

        // 文件输出
        let file_appender = tracing_appender::rolling::hourly(&config.log_dir, &config.log_file);
        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

        // 标准控制台输出
        let (std_non_blocking, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

        let logger = Registry::default()
            .with(EnvFilter::from_default_env().add_directive(log_env.into()))
            .with(
                fmt::Layer::default()
                    .with_writer(std_non_blocking)
                    .event_format(format.clone())
                    .pretty(),
            )
            .with(
                fmt::Layer::default()
                    .with_writer(non_blocking)
                    .event_format(format),
            );

        tracing::subscriber::set_global_default(logger)?;

        Ok(Logger {
            _file_guard: file_guard,
            _stdout_guard: stdout_guard,
        })
    }
}

#[cfg(target_os = "windows")]
fn get_log_format() -> Format<Compact, LocalTime<Rfc3339>> {
    fmt::format()
        .with_level(true) // don't include levels in formatted output
        .with_target(true) // don't include targets
        .with_thread_ids(true)
        // include the thread ID of the current thread
        // .with_thread_names(true)
        // .with_file(true)
        // .with_ansi(true)
        // .with_line_number(true) // include the name of the current thread
        .with_timer(LocalTime::rfc_3339()) // use RFC 3339 timestamps
        .compact()
}
#[cfg(not(target_os = "windows"))]
fn get_log_format() -> Format<Compact> {
    fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .compact()
}
