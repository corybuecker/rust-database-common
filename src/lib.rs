pub use deadpool_postgres::GenericClient;
use deadpool_postgres::{
    CreatePoolError, ManagerConfig, Pool, PoolError, Runtime, tokio_postgres::NoTls,
};
use thiserror::Error;

/// A wrapper around a database URL that prevents the value from being
/// accidentally exposed via `Debug` or `Display`.
#[derive(Clone)]
struct DatabaseUrl(String);

impl DatabaseUrl {
    fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for DatabaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DatabaseUrl(REDACTED)")
    }
}

impl std::fmt::Display for DatabaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DatabaseUrl(REDACTED)")
    }
}

#[derive(Debug, Clone)]
pub struct DatabasePool {
    url: DatabaseUrl,
    pub pool: Option<Pool>,
}

#[derive(Error, Debug)]
pub enum DatabasePoolError {
    #[error("Error creating pool")]
    PoolCreationError(#[from] CreatePoolError),

    #[error("Pool error")]
    PoolError(#[from] PoolError),

    #[error("Pool not initialized")]
    NoPoolError,
}

impl DatabasePool {
    /// Create a new DatabasePool with the given URL.
    pub fn new(url: String) -> Self {
        DatabasePool {
            url: DatabaseUrl(url),
            pool: None,
        }
    }

    /// Connects to the database and initializes the pool.
    pub async fn connect(&mut self) -> Result<(), DatabasePoolError> {
        let config = deadpool_postgres::Config {
            url: Some(self.url.expose().to_owned()),
            manager: Some(ManagerConfig {
                recycling_method: deadpool_postgres::RecyclingMethod::Verified,
            }),
            ..Default::default()
        };

        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(DatabasePoolError::PoolCreationError)?;

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
    #[test]
    fn test_database_pool_secret() {
        let secret = "postgres://user:password@localhost/db";
        let pool = super::DatabasePool::new(secret.to_string());
        let debug_output = format!("{:?}", pool.url);

        assert!(debug_output.contains("REDACTED"));
        assert!(!debug_output.contains(secret));
        assert!(!debug_output.contains("password"));
    }

    #[test]
    fn test_database_pool_debug_secret() {
        let secret = "postgres://user:password@localhost/db";
        let pool = super::DatabasePool::new(secret.to_string());
        let debug_output = format!("{:?}", pool);

        assert!(debug_output.contains("REDACTED"));
        assert!(!debug_output.contains(secret));
        assert!(!debug_output.contains("password"));
    }
}
