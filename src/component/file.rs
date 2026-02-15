use crate::database::SqliteError;
use crate::prelude::*;

#[allow(dead_code)]
pub struct File {
    pub id: i64,
    pub name: String,
    pub path: String,
}

impl File {
    pub fn setup(db: &Database) -> Result<(), Error> {
        db.execute_batch(
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
        .context("failed to create files table")
    }

    fn from_row(row: &Row) -> Result<Self, SqliteError> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
        })
    }

    pub fn new(db: &Database, parent_path: &Path, source_path: &Path) -> Result<File, Error> {
        let name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("invalid file path")?;

        let path = parent_path
            .iter()
            .next_back()
            .context("invalid file path")?
            .to_str();

        let data = fs::read(source_path).context("failed to read file")?;

        db.query_one(
            "INSERT INTO files (name, path, data) VALUES (?, ?, ?) RETURNING id, name, path",
            (name, path, data),
            File::from_row,
        )
        .context("failed to insert file into database")
    }

    pub fn by_path_and_name(db: &Database, path: &str, name: &str) -> Result<File, Error> {
        db.query_one(
            "SELECT id, name, path, data FROM files WHERE path = ? AND name = ?",
            (path, name),
            File::from_row,
        )
        .context("failed to query file from database")
    }

    pub fn get_data(&self, db: &Database) -> Result<Vec<u8>, Error> {
        db.query_one("SELECT data FROM files WHERE id = ?", [self.id], |row| {
            row.get(0)
        })
        .context("failed to query file data from database")
    }

    pub fn delete_all(db: &Database) -> Result<(), Error> {
        db.execute("DELETE FROM files", [])
            .context("failed to delete all files from database")
    }
}

pub async fn get_style(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    println!("GET style {}", name);
    get(db, "styles", &name).into_response()
}

pub async fn get_file(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    println!("GET file {}", name);
    get(db, "files", &name).into_response()
}

pub async fn get_asset(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(name): ax::Path<String>,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    println!("GET asset {}", name);
    get(db, "assets", &name).into_response()
}

fn get(db: &Database, path: &str, name: &str) -> impl IntoResponse {
    match File::by_path_and_name(db, path, name) {
        Ok(file) => {
            let content_type = mime_guess::from_path(name).first_or_octet_stream();

            let header = ax::HeaderMap::from_iter(vec![(
                ax::header::CONTENT_TYPE,
                content_type.to_string().parse().unwrap(),
            )]);

            let data = match file.get_data(db) {
                Ok(data) => data,
                Err(_) => return make_error(500, "Failed to get file data").into_response(),
            };

            (header, data).into_response()
        }
        Err(_) => make_error(404, "File not found").into_response(),
    }
}
