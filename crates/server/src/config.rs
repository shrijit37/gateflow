//! Server configuration from environment variables.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub allowed_origins: Vec<String>,
    pub body_limit_bytes: usize,
    pub peek_prefix_bytes: usize,
    pub default_connect_timeout: Duration,
    pub default_timeout: Duration,
}

impl Config {
    pub fn from_env() -> Self {
        let bind = env_or("GATEFLOW_BIND", "0.0.0.0:8080");
        let database_url = env_or("DATABASE_URL", "sqlite://gateflow.db");
        let allowed_origins = env_or("GATEFLOW_CORS_ORIGINS", "http://localhost:3000")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let body_limit_bytes = env_or("GATEFLOW_BODY_LIMIT_MB", "100")
            .parse::<usize>()
            .unwrap_or(100)
            * 1024
            * 1024;
        let peek_prefix_bytes = env_or("GATEFLOW_PEEK_BYTES", "4096")
            .parse()
            .unwrap_or(4096);
        let default_connect_timeout = Duration::from_millis(
            env_or("GATEFLOW_CONNECT_TIMEOUT_MS", "10000")
                .parse()
                .unwrap_or(10_000),
        );
        let default_timeout = Duration::from_millis(
            env_or("GATEFLOW_TIMEOUT_MS", "120000")
                .parse()
                .unwrap_or(120_000),
        );
        Config {
            bind,
            database_url,
            allowed_origins,
            body_limit_bytes,
            peek_prefix_bytes,
            default_connect_timeout,
            default_timeout,
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}