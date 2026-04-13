pub use deadpool_postgres::GenericClient;
use deadpool_postgres::{
    CreatePoolError, ManagerConfig, Pool, PoolError, Runtime, tokio_postgres::NoTls,
};
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use thiserror::Error;

/// A wrapper around a database URL that prevents the value from being
/// accidentally exposed via `Debug` or `Display`.
#[derive(Clone)]
struct SensitiveString(String);

impl SensitiveString {
    fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitiveString(REDACTED)")
    }
}

impl std::fmt::Display for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SensitiveString(REDACTED)")
    }
}

#[derive(Debug, Clone)]
enum SslMode {
    Disable,
    Require { ca_cert_pem: SensitiveString },
}

#[derive(Debug, Clone)]
pub struct DatabasePool {
    url: SensitiveString,
    pool: Option<Pool>,
    ssl_mode: SslMode,
}

#[derive(Error, Debug)]
pub enum DatabasePoolError {
    #[error("Error creating pool")]
    PoolCreationError(#[from] CreatePoolError),

    #[error("TLS error")]
    TlsError(#[from] native_tls::Error),

    #[error("Pool error")]
    PoolError(#[from] PoolError),

    #[error("Pool not initialized")]
    NoPoolError,
}

impl DatabasePool {
    pub fn new(url: String) -> Self {
        DatabasePool {
            url: SensitiveString(url),
            pool: None,
            ssl_mode: SslMode::Disable,
        }
    }

    pub fn with_required_ssl_mode(mut self, ca_cert_pem: String) -> Self {
        self.ssl_mode = SslMode::Require {
            ca_cert_pem: SensitiveString(ca_cert_pem),
        };
        self.pool = None;
        self
    }

    pub async fn connect(&mut self) -> Result<(), DatabasePoolError> {
        let config = deadpool_postgres::Config {
            url: Some(self.url.expose().to_owned()),
            manager: Some(ManagerConfig {
                recycling_method: deadpool_postgres::RecyclingMethod::Verified,
            }),
            ..Default::default()
        };

        let pool = match &self.ssl_mode {
            SslMode::Disable => config.create_pool(Some(Runtime::Tokio1), NoTls),
            SslMode::Require { ca_cert_pem } => {
                let ca_certificate = Certificate::from_pem(ca_cert_pem.expose().as_bytes())
                    .map_err(DatabasePoolError::TlsError)?;

                let native_tls_connector = TlsConnector::builder()
                    .add_root_certificate(ca_certificate)
                    .build()
                    .map_err(DatabasePoolError::TlsError)?;

                config.create_pool(
                    Some(Runtime::Tokio1),
                    MakeTlsConnector::new(native_tls_connector),
                )
            }
        };

        let pool = pool.map_err(DatabasePoolError::PoolCreationError)?;

        // Check connectivity by getting a client from the pool.
        // This ensures the pool is valid before storing it.
        let _ = pool.get().await.map_err(DatabasePoolError::PoolError)?;

        self.pool = Some(pool);

        Ok(())
    }

    pub async fn get_client(&self) -> Result<impl GenericClient, DatabasePoolError> {
        if let Some(pool) = &self.pool {
            let client = pool.get().await.map_err(DatabasePoolError::PoolError)?;
            Ok(client)
        } else {
            Err(DatabasePoolError::NoPoolError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatabasePool, DatabasePoolError, SslMode};

    #[test]
    fn test_database_pool_secret() {
        let secret = "postgres://user:password@localhost/db";
        let pool = DatabasePool::new(secret.to_string());
        let debug_output = format!("{:?}", pool.url);

        assert!(debug_output.contains("REDACTED"));
        assert!(!debug_output.contains(secret));
        assert!(!debug_output.contains("password"));
    }

    #[test]
    fn test_database_pool_debug_secret() {
        let secret = "postgres://user:password@localhost/db";
        let pool = DatabasePool::new(secret.to_string());
        let debug_output = format!("{:?}", pool);

        assert!(debug_output.contains("REDACTED"));
        assert!(!debug_output.contains(secret));
        assert!(!debug_output.contains("password"));
    }

    #[test]
    fn test_database_pool_display_secret() {
        let secret = "postgres://user:password@localhost/db";
        let pool = DatabasePool::new(secret.to_string());
        let display_output = format!("{}", pool.url);

        assert_eq!(display_output, "SensitiveString(REDACTED)");
        assert!(!display_output.contains(secret));
        assert!(!display_output.contains("password"));
    }

    #[test]
    fn test_new_defaults_to_ssl_disable() {
        let pool = DatabasePool::new("postgres://localhost/db".to_string());
        assert!(matches!(pool.ssl_mode, SslMode::Disable));
        assert!(pool.pool.is_none());
    }

    #[test]
    fn test_with_ssl_mode_returns_new_pool_with_mode() {
        let original = DatabasePool::new("postgres://localhost/db".to_string());
        let cert = "-----BEGIN CERTIFICATE-----\nabc\n-----END CERTIFICATE-----".to_string();

        let updated = original.clone().with_required_ssl_mode(cert.clone());

        assert!(matches!(original.ssl_mode, SslMode::Disable));
        assert!(matches!(
            updated.ssl_mode,
            SslMode::Require { ca_cert_pem } if ca_cert_pem.expose() == cert
        ));
        assert!(updated.pool.is_none());
    }

    #[test]
    fn test_no_pool_error_display() {
        assert_eq!(
            DatabasePoolError::NoPoolError.to_string(),
            "Pool not initialized"
        );
    }
}
