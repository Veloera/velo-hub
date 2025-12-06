use std::path::PathBuf;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Directory for log files
    pub log_dir: PathBuf,
    /// Enable console logging
    pub console_enabled: bool,
    /// Enable file logging
    pub file_enabled: bool,
    /// Enable JSON format for file logs
    pub json_format: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("velo-hub")
            .join("logs");

        Self {
            level: "info".to_string(),
            log_dir,
            console_enabled: true,
            file_enabled: true,
            json_format: false,
        }
    }
}

/// Initialize the logging system with file rotation
pub fn init_logging(config: LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Create log directory if it doesn't exist
    if config.file_enabled {
        std::fs::create_dir_all(&config.log_dir)?;
    }

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(env_filter);

    // Console layer
    let console_layer = if config.console_enabled {
        Some(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_filter(EnvFilter::new(&config.level)),
        )
    } else {
        None
    };

    // File layer with daily rotation
    if config.file_enabled {
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("velo-hub")
            .filename_suffix("log")
            .max_log_files(30) // Keep 30 days of logs
            .build(&config.log_dir)?;

        let file_layer = fmt::layer()
            .with_writer(file_appender)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .with_filter(EnvFilter::new(&config.level));

        // Initialize subscriber with layers
        registry.with(console_layer).with(file_layer).try_init()?;
    } else {
        // Initialize subscriber with console layer only
        registry.with(console_layer).try_init()?;
    }

    tracing::info!(
        log_dir = %config.log_dir.display(),
        level = %config.level,
        console_enabled = config.console_enabled,
        file_enabled = config.file_enabled,
        "Logging system initialized"
    );

    Ok(())
}

/// Log levels for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    pub fn as_tracing_level(&self) -> Level {
        match self {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl From<LogLevel> for String {
    fn from(level: LogLevel) -> Self {
        level.as_str().to_string()
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Debug.as_str(), "debug");

        let level: LogLevel = "info".parse().unwrap();
        assert_eq!(level, LogLevel::Info);
    }
}
