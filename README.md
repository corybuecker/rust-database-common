# rust-database-common

`rust-database-common` provides a simple, reusable abstraction for database connection pooling in my other Rust applications.

## Intent

The intent of this crate is to offer a lightweight, common interface for managing database connection pools, so I can focus on my application logic rather than connection management.

## Features

- Simple API for creating and managing a database connection pool
- Built on top of [`deadpool-postgres`](https://crates.io/crates/deadpool-postgres) and [`tokio-postgres`](https://crates.io/crates/tokio-postgres)
- Error handling with [`thiserror`](https://crates.io/crates/thiserror)
- Asynchronous connection management

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
rust-database-common = { git = "https://github.com/corybuecker/rust-database-common", branch = "main" }
```

### Example

```rust
use rust_database_common::DatabasePool;

#[tokio::main]
async fn main() {
    let mut db_pool = DatabasePool::new("postgres://user:password@localhost/dbname".to_string());
    db_pool.connect().await.expect("Failed to connect to database");
    let client = db_pool.get_client().await.expect("Failed to get client");
    // Use `client` as a regular tokio_postgres::Client
}
```

## Notes

- This README was written by AI.

## License

This project is licensed under the MIT License.