use crate::prelude::*;

#[allow(dead_code)]
pub struct File {
    pub id: i64,
    pub name: String,
    pub path: String,
}

impl File {
    pub async fn setup(db: &Database) {
        sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS files (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    path TEXT NOT NULL,
                    data BLOB NOT NULL
                );

                CREATE INDEX IF NOT EXISTS files_id_index ON files (id);
                CREATE INDEX IF NOT EXISTS files_name_index ON files (name);
                CREATE INDEX IF NOT EXISTS files_path_index ON files (path);
            "#,
        )
        .execute(&db.pool)
        .await
        .expect("failed to create files table");
    }

    fn from_row(row: sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get(0),
            name: row.get(1),
            path: row.get(2),
        }
    }

    pub async fn new(db: &Database, parent_path: &Path, source_path: &Path) -> File {
        let name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("invalid file path");

        let path = parent_path
            .iter()
            .next_back()
            .expect("invalid file path")
            .to_str()
            .unwrap();

        let data = fs::read(source_path).expect("failed to read file");

        let record =
            sqlx::query("INSERT INTO files (name, path, data) VALUES (?, ?, ?) RETURNING id")
                .bind(name)
                .bind(path)
                .bind(data)
                .fetch_one(&db.pool)
                .await
                .expect("failed to insert file into database");

        File {
            id: record.get(0),
            name: name.to_string(),
            path: path.to_string(),
        }
    }

    pub async fn by_path_and_name(db: &Database, path: &str, name: &str) -> Option<File> {
        sqlx::query("SELECT id, name, path, data FROM files WHERE path = ? AND name = ?")
            .bind(path)
            .bind(name)
            .fetch_optional(&db.pool)
            .await
            .expect("failed to query file from database")
            .map(File::from_row)
    }

    pub async fn get_data(&self, db: &Database) -> Vec<u8> {
        sqlx::query("SELECT data FROM files WHERE id = ?")
            .bind(self.id)
            .fetch_one(&db.pool)
            .await
            .expect("failed to query file data from database")
            .get(0)
    }

    pub async fn delete_all(db: &Database) {
        sqlx::query("DELETE FROM files")
            .execute(&db.pool)
            .await
            .expect("failed to delete all files from database");
    }
}

pub async fn get_style(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db;
    println!("GET style {}", name);
    get(db, "styles", &name).await.into_response()
}

pub async fn get_file(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db;
    println!("GET file {}", name);
    get(db, "files", &name).await.into_response()
}

pub async fn get_asset(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db;
    println!("GET asset {}", name);
    get(db, "assets", &name).await.into_response()
}

async fn get(db: &Database, path: &str, name: &str) -> impl IntoResponse {
    match File::by_path_and_name(db, path, name).await {
        Some(file) => {
            let content_type = mime_guess::from_path(name).first_or_octet_stream();

            let header = ax::HeaderMap::from_iter(vec![(
                ax::header::CONTENT_TYPE,
                content_type.to_string().parse().unwrap(),
            )]);

            (header, file.get_data(db).await).into_response()
        }
        None => ax::StatusCode::NOT_FOUND.into_response(),
    }
}
