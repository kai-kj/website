pub struct Database {
    pub pool: sqlx::SqlitePool,
}

impl Database {
    pub async fn connect(db_path: &str) -> Database {
        let connection_options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = sqlx::SqlitePool::connect_with(connection_options)
            .await
            .expect("failed to connect to database");

        Database { pool }
    }
}
