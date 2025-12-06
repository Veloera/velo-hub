use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub session: SessionConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: f64,
    pub window_duration_secs: u64,
    pub cooldown_duration_secs: u64,
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 0.5,
            window_duration_secs: 60,
            cooldown_duration_secs: 30,
            half_open_max_requests: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub ttl_minutes: u64,
    pub cleanup_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                enable_tls: false,
            },
            database: DatabaseConfig {
                path: "~/.velo-hub/data.db".to_string(),
            },
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 0.5,
                window_duration_secs: 60,
                cooldown_duration_secs: 30,
                half_open_max_requests: 3,
            },
            session: SessionConfig {
                ttl_minutes: 5,
                cleanup_interval_secs: 60,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: Some("~/.velo-hub/logs/app.log".to_string()),
            },
        }
    }
}

impl AppConfig {
    /// Load configuration with the following precedence (highest to lowest):
    /// 1. Environment variables
    /// 2. Config file at ~/.velo-hub/config.toml
    /// 3. Default configuration
    pub fn load() -> Result<Self, ConfigError> {
        // Start with default configuration
        let mut config = Self::default();

        // Try to load from config file
        if let Some(config_path) = Self::get_config_path() {
            if config_path.exists() {
                tracing::info!("Loading configuration from: {}", config_path.display());
                match Self::load_from_file(&config_path) {
                    Ok(file_config) => {
                        config = file_config;
                        tracing::info!("Configuration loaded successfully from file");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load config file, using defaults: {}", e);
                    }
                }
            } else {
                tracing::info!(
                    "Config file not found at {}, using defaults",
                    config_path.display()
                );
            }
        }

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Get the default configuration file path
    fn get_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".velo-hub").join("config.toml"))
    }

    /// Load configuration from a TOML file
    fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(path.clone(), e.to_string()))?;

        let config: AppConfig =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

        Ok(config)
    }

    /// Apply environment variable overrides to the configuration
    fn apply_env_overrides(&mut self) {
        // Server configuration
        if let Ok(host) = env::var("VELO_HUB_SERVER_HOST") {
            tracing::info!("Overriding server.host from environment: {}", host);
            self.server.host = host;
        }
        if let Ok(port) = env::var("VELO_HUB_SERVER_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                tracing::info!("Overriding server.port from environment: {}", port_num);
                self.server.port = port_num;
            } else {
                tracing::warn!("Invalid port number in VELO_HUB_SERVER_PORT: {}", port);
            }
        }
        if let Ok(enable_tls) = env::var("VELO_HUB_SERVER_ENABLE_TLS") {
            if let Ok(tls_bool) = enable_tls.parse::<bool>() {
                tracing::info!(
                    "Overriding server.enable_tls from environment: {}",
                    tls_bool
                );
                self.server.enable_tls = tls_bool;
            }
        }

        // Database configuration
        if let Ok(db_path) = env::var("VELO_HUB_DATABASE_PATH") {
            tracing::info!("Overriding database.path from environment: {}", db_path);
            self.database.path = db_path;
        }

        // Circuit breaker configuration
        if let Ok(threshold) = env::var("VELO_HUB_CIRCUIT_BREAKER_FAILURE_THRESHOLD") {
            if let Ok(threshold_f64) = threshold.parse::<f64>() {
                tracing::info!(
                    "Overriding circuit_breaker.failure_threshold from environment: {}",
                    threshold_f64
                );
                self.circuit_breaker.failure_threshold = threshold_f64;
            }
        }
        if let Ok(window) = env::var("VELO_HUB_CIRCUIT_BREAKER_WINDOW_DURATION_SECS") {
            if let Ok(window_u64) = window.parse::<u64>() {
                tracing::info!(
                    "Overriding circuit_breaker.window_duration_secs from environment: {}",
                    window_u64
                );
                self.circuit_breaker.window_duration_secs = window_u64;
            }
        }
        if let Ok(cooldown) = env::var("VELO_HUB_CIRCUIT_BREAKER_COOLDOWN_DURATION_SECS") {
            if let Ok(cooldown_u64) = cooldown.parse::<u64>() {
                tracing::info!(
                    "Overriding circuit_breaker.cooldown_duration_secs from environment: {}",
                    cooldown_u64
                );
                self.circuit_breaker.cooldown_duration_secs = cooldown_u64;
            }
        }
        if let Ok(max_requests) = env::var("VELO_HUB_CIRCUIT_BREAKER_HALF_OPEN_MAX_REQUESTS") {
            if let Ok(max_requests_u32) = max_requests.parse::<u32>() {
                tracing::info!(
                    "Overriding circuit_breaker.half_open_max_requests from environment: {}",
                    max_requests_u32
                );
                self.circuit_breaker.half_open_max_requests = max_requests_u32;
            }
        }

        // Session configuration
        if let Ok(ttl) = env::var("VELO_HUB_SESSION_TTL_MINUTES") {
            if let Ok(ttl_u64) = ttl.parse::<u64>() {
                tracing::info!(
                    "Overriding session.ttl_minutes from environment: {}",
                    ttl_u64
                );
                self.session.ttl_minutes = ttl_u64;
            }
        }
        if let Ok(cleanup) = env::var("VELO_HUB_SESSION_CLEANUP_INTERVAL_SECS") {
            if let Ok(cleanup_u64) = cleanup.parse::<u64>() {
                tracing::info!(
                    "Overriding session.cleanup_interval_secs from environment: {}",
                    cleanup_u64
                );
                self.session.cleanup_interval_secs = cleanup_u64;
            }
        }

        // Logging configuration
        if let Ok(level) = env::var("VELO_HUB_LOGGING_LEVEL") {
            tracing::info!("Overriding logging.level from environment: {}", level);
            self.logging.level = level;
        }
        if let Ok(file) = env::var("VELO_HUB_LOGGING_FILE") {
            tracing::info!("Overriding logging.file from environment: {}", file);
            self.logging.file = Some(file);
        }
    }

    /// Save the current configuration to the default config file
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::get_config_path().ok_or_else(|| {
            ConfigError::PathResolution("Unable to determine home directory".to_string())
        })?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::FileWrite(config_path.clone(), e.to_string()))?;
        }

        let toml_string =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;

        fs::write(&config_path, toml_string)
            .map_err(|e| ConfigError::FileWrite(config_path.clone(), e.to_string()))?;

        tracing::info!("Configuration saved to: {}", config_path.display());
        Ok(())
    }

    /// Expand tilde (~) in paths to the user's home directory
    pub fn expand_path(path: &str) -> String {
        if path.starts_with("~/") || path.starts_with("~\\") {
            if let Some(home) = dirs::home_dir() {
                return path.replacen("~", &home.to_string_lossy(), 1);
            }
        }
        path.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {0}: {1}")]
    FileRead(PathBuf, String),

    #[error("Failed to write config file {0}: {1}")]
    FileWrite(PathBuf, String),

    #[error("Failed to parse config: {0}")]
    Parse(String),

    #[error("Failed to serialize config: {0}")]
    Serialize(String),

    #[error("Failed to resolve path: {0}")]
    PathResolution(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.enable_tls, false);
        assert_eq!(config.circuit_breaker.failure_threshold, 0.5);
        assert_eq!(config.session.ttl_minutes, 5);
    }

    #[test]
    fn test_env_override() {
        // Set environment variables
        env::set_var("VELO_HUB_SERVER_HOST", "0.0.0.0");
        env::set_var("VELO_HUB_SERVER_PORT", "9000");
        env::set_var("VELO_HUB_SESSION_TTL_MINUTES", "10");

        let mut config = AppConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.session.ttl_minutes, 10);

        // Clean up
        env::remove_var("VELO_HUB_SERVER_HOST");
        env::remove_var("VELO_HUB_SERVER_PORT");
        env::remove_var("VELO_HUB_SESSION_TTL_MINUTES");
    }

    #[test]
    fn test_toml_serialization() {
        let config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&config).unwrap();

        // Verify it can be deserialized back
        let deserialized: AppConfig = toml::from_str(&toml_string).unwrap();
        assert_eq!(deserialized.server.host, config.server.host);
        assert_eq!(deserialized.server.port, config.server.port);
    }

    #[test]
    fn test_expand_path() {
        let path = "~/test/path";
        let expanded = AppConfig::expand_path(path);

        // Should not contain tilde anymore
        assert!(!expanded.contains('~'));

        let windows_path = "~\\test\\path";
        let expanded_windows = AppConfig::expand_path(windows_path);
        assert!(!expanded_windows.starts_with('~'));

        // Absolute path should remain unchanged
        let abs_path = "/absolute/path";
        let expanded_abs = AppConfig::expand_path(abs_path);
        assert_eq!(expanded_abs, abs_path);
    }

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 0.5);
        assert_eq!(config.window_duration_secs, 60);
        assert_eq!(config.cooldown_duration_secs, 30);
        assert_eq!(config.half_open_max_requests, 3);
    }
}
