use deadpool_postgres::{CreatePoolError, ManagerConfig, Pool, PoolError, Runtime};
use thiserror::Error;
use tokio_postgres::NoTls;
pub use tokio_postgres::types::ToSql;

pub type Client = deadpool_postgres::Client;

#[derive(Debug, Clone)]
pub struct DatabasePool {
    url: String,
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
        DatabasePool { url, pool: None }
    }

    /// Connects to the database and initializes the pool.
    pub async fn connect(&mut self) -> Result<(), DatabasePoolError> {
        let config = deadpool_postgres::Config {
            url: Some(self.url.clone()),
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

    pub async fn get_client(&self) -> Result<deadpool_postgres::Client, DatabasePoolError> {
        if let Some(pool) = &self.pool {
            pool.get().await.map_err(DatabasePoolError::PoolError)
        } else {
            Err(DatabasePoolError::NoPoolError)
        }
    }
}

// Legacy code for handling database connection retries manually.

// async fn database_connection_handler(client: Arc<RwLock<Client>>) {
//     // Get the database URL from the environment variable
//     let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

//     loop {
//         // Try to connect to the database
//         let (replacement_client, connection) = match connect(&database_url, NoTls).await {
//             Ok((client, connection)) => {
//                 info!("Connected to database");
//                 (client, connection)
//             }
//             // If connection fails, log the error and retry after 5 seconds
//             Err(e) => {
//                 tracing::error!("Failed to connect to database: {}", e);
//                 sleep_until(tokio::time::Instant::now() + Duration::from_secs(5)).await;
//                 continue;
//             }
//         };

//         // Replace the current client with the new one.
//         // Acquire a write lock on the Arc-wrapped RwLock<Client> to ensure exclusive access,
//         // so that no other task is reading or writing to the client while we update it.
//         let mut guard = client.write().await;

//         // Overwrite the existing client with the newly established replacement_client.
//         // This allows the rest of the application to transparently use the new connection
//         // without needing to restart or reinitialize any consumers of the client.
//         *guard = replacement_client;

//         // Explicitly drop the guard to release the write lock as soon as possible,
//         // allowing other tasks to acquire the lock and use the updated client.
//         drop(guard);

//         // Wait for the connection to finish, log errors if any, and loop to reconnect
//         if let Err(e) = connection.await {
//             error!("Connection error: {}", e);
//             continue;
//         }
//     }
// }
