use crate::prelude::*;

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

    pub async fn delete_all(db: &Database) {
        sqlx::query("DELETE FROM files")
            .execute(&db.pool)
            .await
            .expect("failed to delete all files from database");
    }

    pub async fn new(db: &Database, parent_path: &Path, source_path: &Path) -> File {
        let name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("invalid file path");

        let path = parent_path
            .iter()
            .last()
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

    pub async fn from_folder(db: &Database, folder_path: &Path) -> Vec<File> {
        let mut files = Vec::new();

        for parent in fs::read_dir(folder_path).expect("failed to read files directory") {
            let parent = parent.unwrap();
            for entry in fs::read_dir(&parent.path()).expect("failed to read files directory") {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    let file = File::new(db, &parent.path(), &entry.path()).await;
                    files.push(file);
                }
            }
        }

        files
    }
}
