use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub cognito: CognitoConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub api_base_url: String,
    pub app_base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognitoConfig {
    pub user_pool_id: String,
    pub client_id: String,
    pub domain: String,
    pub issuer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingEnv { name: &'static str },
    InvalidEnv { name: &'static str, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnv { name } => write!(f, "missing required environment variable {name}"),
            Self::InvalidEnv { name, reason } => {
                write!(f, "invalid environment variable {name}: {reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(
        lookup: impl Fn(&'static str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            database: database_config(&lookup)?,
            api: ApiConfig {
                api_base_url: required_value(&lookup, "API_BASE_URL")?,
                app_base_url: required_value(&lookup, "APP_BASE_URL")?,
            },
            cognito: CognitoConfig {
                user_pool_id: required_value(&lookup, "COGNITO_USER_POOL_ID")?,
                client_id: required_value(&lookup, "COGNITO_CLIENT_ID")?,
                domain: required_value(&lookup, "COGNITO_DOMAIN")?,
                issuer: required_value(&lookup, "COGNITO_ISSUER")?,
            },
        })
    }
}

fn database_config(
    lookup: &impl Fn(&'static str) -> Option<String>,
) -> Result<DatabaseConfig, ConfigError> {
    Ok(DatabaseConfig {
        host: required_value(lookup, "DB_HOST")?,
        port: database_port(lookup)?,
        name: required_value(lookup, "DB_NAME")?,
        username: required_value(lookup, "DB_USERNAME")?,
        password: required_value(lookup, "DB_PASSWORD")?,
    })
}

fn database_port(lookup: &impl Fn(&'static str) -> Option<String>) -> Result<u16, ConfigError> {
    optional_value(lookup, "DB_PORT")
        .map(|port| {
            port.parse::<u16>().map_err(|_| ConfigError::InvalidEnv {
                name: "DB_PORT",
                reason: "must be an integer TCP port".to_string(),
            })
        })
        .transpose()
        .map(|port| port.unwrap_or(5432))
}

fn required_value(
    lookup: &impl Fn(&'static str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    optional_value(lookup, name).ok_or(ConfigError::MissingEnv { name })
}

fn optional_value(
    lookup: &impl Fn(&'static str) -> Option<String>,
    name: &'static str,
) -> Option<String> {
    lookup(name).and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;

    use super::{AppConfig, ConfigError};

    fn base_env() -> HashMap<&'static str, String> {
        HashMap::from([
            ("DB_HOST", "db.internal".to_string()),
            ("DB_NAME", "linkdrop".to_string()),
            ("DB_USERNAME", "linkdrop_app".to_string()),
            ("DB_PASSWORD", "secret".to_string()),
            ("API_BASE_URL", "https://api.linkdrop.ahara.io".to_string()),
            ("APP_BASE_URL", "https://linkdrop.ahara.io".to_string()),
            ("COGNITO_USER_POOL_ID", "us-east-1_pool".to_string()),
            ("COGNITO_CLIENT_ID", "linkdrop-app-client".to_string()),
            ("COGNITO_DOMAIN", "auth.services.ahara.io".to_string()),
            (
                "COGNITO_ISSUER",
                "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_pool".to_string(),
            ),
        ])
    }

    fn load(env: &HashMap<&'static str, String>) -> Result<AppConfig, ConfigError> {
        AppConfig::from_lookup(|name| env.get(name).cloned())
    }

    #[test]
    fn config_loads_required_runtime_values() {
        let mut env = base_env();
        env.insert("DB_PORT", "6543".to_string());

        let config = load(&env).unwrap();

        assert_eq!(config.database.host, "db.internal");
        assert_eq!(config.database.port, 6543);
        assert_eq!(config.database.name, "linkdrop");
        assert_eq!(config.api.api_base_url, "https://api.linkdrop.ahara.io");
        assert_eq!(config.cognito.client_id, "linkdrop-app-client");
    }

    #[test]
    fn config_defaults_database_port() {
        let config = load(&base_env()).unwrap();

        assert_eq!(config.database.port, 5432);
    }

    #[test]
    fn config_reports_missing_required_values() {
        let mut env = base_env();
        env.remove("DB_HOST");

        assert_eq!(load(&env), Err(ConfigError::MissingEnv { name: "DB_HOST" }));
    }

    #[test]
    fn config_rejects_invalid_database_port() {
        let mut env = base_env();
        env.insert("DB_PORT", "not-a-port".to_string());

        assert!(matches!(
            load(&env),
            Err(ConfigError::InvalidEnv {
                name: "DB_PORT",
                ..
            })
        ));
    }
}
