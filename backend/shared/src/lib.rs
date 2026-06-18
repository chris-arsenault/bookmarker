pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod library;
pub mod library_pg;
mod library_pg_capture_helpers;
pub mod processing;
pub mod url_normalization;

pub const SERVICE_NAME: &str = "linkdrop";
pub const PROJECT_KEY: &str = "linkdrop";
pub const APP_CLIENT_KEY: &str = "linkdrop-app";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkdropConfig {
    pub service_name: &'static str,
    pub project_key: &'static str,
    pub app_client_key: &'static str,
}

impl Default for LinkdropConfig {
    fn default() -> Self {
        Self {
            service_name: SERVICE_NAME,
            project_key: PROJECT_KEY,
            app_client_key: APP_CLIENT_KEY,
        }
    }
}

pub fn service_name() -> &'static str {
    SERVICE_NAME
}

#[cfg(test)]
mod tests {
    use super::{service_name, LinkdropConfig, APP_CLIENT_KEY, PROJECT_KEY, SERVICE_NAME};

    #[test]
    fn exposes_linkdrop_platform_identity() {
        let config = LinkdropConfig::default();

        assert_eq!(service_name(), SERVICE_NAME);
        assert_eq!(config.service_name, "linkdrop");
        assert_eq!(config.project_key, PROJECT_KEY);
        assert_eq!(config.app_client_key, APP_CLIENT_KEY);
    }
}
