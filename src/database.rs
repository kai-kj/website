use rusqlite::{Connection, Params};
pub use rusqlite::{Error as SqliteError, Row};

use crate::prelude::*;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn connect(db_path: &str) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::open(db_path).context("failed to open database")?,
        })
    }

    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<(), Error> {
        self.connection
            .prepare(sql)
            .context("failed to prepare SQL")?
            .execute(params)
            .map(|_| ())
            .context("failed to execute SQL")
    }

    pub fn execute_batch(&self, sql: &str) -> Result<(), Error> {
        self.connection
            .execute_batch(sql)
            .context("failed to execute batch SQL")
    }

    pub fn query_one<P: Params, F: FnMut(&Row<'_>) -> Result<T, SqliteError>, T>(
        &self,
        sql: &str,
        params: P,
        f: F,
    ) -> Result<T, Error> {
        self.query_mul(sql, params, f)?
            .into_iter()
            .next()
            .ok_or(Error::new("no rows returned"))
    }

    pub fn query_mul<P: Params, F: FnMut(&Row<'_>) -> Result<T, SqliteError>, T>(
        &self,
        sql: &str,
        params: P,
        f: F,
    ) -> Result<Vec<T>, Error> {
        self.connection
            .prepare(sql)
            .context("failed to prepare SQL")?
            .query_map(params, f)
            .context("failed to execute SQL")?
            .map(|row| row.context("failed to map row"))
            .collect()
    }
}
