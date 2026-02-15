use crate::database::SqliteError;
use crate::prelude::*;

pub struct Asset {
    pub id: i64,
    pub name: String,
}

impl Asset {
    pub fn setup(db: &Database) -> Result<(), Error> {
        db.execute_batch(
            r#"
                CREATE TABLE IF NOT EXISTS styles (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    data BLOB NOT NULL
                );

                CREATE TABLE IF NOT EXISTS posts_assets (
                    post_id TEXT NOT NULL,
                    asset_id INTEGER NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
                    FOREIGN KEY (asset_id) REFERENCES styles (id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS assets_id_index ON styles (id);
                CREATE INDEX IF NOT EXISTS assets_name_index ON styles (name);
            "#,
        )
        .context("failed to create styles table")
    }

    fn from_row(row: &Row) -> Result<Self, SqliteError> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }

    pub fn new(db: &Database, path: &Path) -> Result<Self, Error> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("invalid asset path")?;

        let data = fs::read(path).context("failed to read asset file")?;

        db.query_one(
            "INSERT INTO styles (name, data) VALUES (?, ?) RETURNING id, name",
            (name, data),
            Asset::from_row,
        )
        .context("failed to insert asset into database")
    }

    pub fn by_post_and_name(
        db: &Database,
        post_name: &str,
        asset_name: &str,
    ) -> Result<Self, Error> {
        db.query_one(
            r#"
                SELECT styles.id, styles.name
                FROM styles
                JOIN posts_assets ON styles.id = posts_assets.asset_id
                WHERE posts_assets.post_id = ? AND styles.name = ?;
            "#,
            (post_name, asset_name),
            Asset::from_row,
        )
        .context("failed to query asset by post name and asset name from database")
    }

    pub fn get_data(&self, db: &Database) -> Result<Vec<u8>, Error> {
        db.query_one("SELECT data FROM styles WHERE id = ?;", [self.id], |row| {
            row.get(0)
        })
        .context("failed to query data from database")
    }

    pub fn delete_all(db: &Database) -> Result<(), Error> {
        db.execute("DELETE FROM styles", [])
            .context("failed to delete all styles from database")
    }
}

pub async fn get_asset(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path((post, name)): ax::Path<(String, String)>,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();

    println!("GET asset {}/{}", post, name);

    let asset = match Asset::by_post_and_name(db, &post, &name) {
        Ok(asset) => asset,
        Err(_) => return make_error(404, "Asset not found").into_response(),
    };

    let content_type = mime_guess::from_path(&asset.name).first_or_octet_stream();

    let header = ax::HeaderMap::from_iter(vec![(
        ax::header::CONTENT_TYPE,
        content_type.to_string().parse().unwrap(),
    )]);

    let data = match asset.get_data(db) {
        Ok(data) => data,
        Err(_) => return make_error(500, "Failed to get asset data").into_response(),
    };

    (header, data).into_response()
}
