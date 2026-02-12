use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BrpConfig {
    pub endpoint: String,
    pub timeout: Duration,
}

impl Default for BrpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:15721".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
}

impl BrpConfig {
    pub fn new(endpoint: impl Into<String>, timeout: Duration) -> Self {
        Self {
            endpoint: endpoint.into(),
            timeout,
        }
    }

    pub fn from_env() -> Self {
        let endpoint =
            std::env::var("BRP_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:15721".to_string());

        let timeout = std::env::var("BRP_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_secs(30));

        Self { endpoint, timeout }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BrpConfig::default();
        assert_eq!(config.endpoint, "http://127.0.0.1:15721");
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_new_config() {
        let config = BrpConfig::new("http://localhost:8080", Duration::from_secs(10));
        assert_eq!(config.endpoint, "http://localhost:8080");
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_from_env_defaults() {
        std::env::remove_var("BRP_ENDPOINT");
        std::env::remove_var("BRP_TIMEOUT_MS");

        let config = BrpConfig::from_env();
        assert_eq!(config.endpoint, "http://127.0.0.1:15721");
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_from_env_custom() {
        std::env::set_var("BRP_ENDPOINT", "http://custom:9999");
        std::env::set_var("BRP_TIMEOUT_MS", "5000");

        let config = BrpConfig::from_env();
        assert_eq!(config.endpoint, "http://custom:9999");
        assert_eq!(config.timeout, Duration::from_millis(5000));

        std::env::remove_var("BRP_ENDPOINT");
        std::env::remove_var("BRP_TIMEOUT_MS");
    }
}
