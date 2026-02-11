use crate::prelude::*;

pub struct Asset {
    pub id: i64,
    pub name: String,
}

impl Asset {
    pub async fn setup(db: &Database) {
        sqlx::query(
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
        .execute(&db.pool)
        .await
        .expect("failed to create styles table");
    }

    pub async fn new(db: &Database, path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("invalid asset path");

        let data = fs::read(path).expect("failed to read asset file");

        let record = sqlx::query("INSERT INTO styles (name, data) VALUES (?, ?) RETURNING id")
            .bind(name)
            .bind(data)
            .fetch_one(&db.pool)
            .await
            .expect("failed to insert asset into database");

        Asset {
            id: record.get(0),
            name: name.to_string(),
        }
    }

    pub async fn by_post_and_name(
        db: &Database,
        post_name: &str,
        asset_name: &str,
    ) -> Option<Asset> {
        sqlx::query(
            r#"
                SELECT styles.id, styles.name
                FROM styles
                JOIN posts_assets ON styles.id = posts_assets.asset_id
                WHERE posts_assets.post_id = ? AND styles.name = ?;
            "#,
        )
        .bind(post_name)
        .bind(asset_name)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query asset by post name and asset name from database")
        .map(|row| Asset {
            id: row.get(0),
            name: row.get(1),
        })
    }

    pub async fn get_data(&self, db: &Database) -> Vec<u8> {
        sqlx::query("SELECT data FROM styles WHERE id = ?;")
            .bind(&self.id)
            .fetch_one(&db.pool)
            .await
            .expect("failed to query data from database")
            .get(0)
    }

    pub async fn delete_all(db: &Database) {
        sqlx::query("DELETE FROM styles")
            .execute(&db.pool)
            .await
            .expect("failed to delete all styles from database");
    }
}

pub async fn get_asset(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path((post, name)): ax::Path<(String, String)>,
) -> (ax::StatusCode, ax::HeaderMap, Vec<u8>) {
    let db = &state.db;

    println!("GET asset {}/{}", post, name);

    let asset = match Asset::by_post_and_name(db, &post, &name).await {
        Some(asset) => asset,
        None => return (ax::StatusCode::NOT_FOUND, ax::HeaderMap::new(), vec![]),
    };

    let content_type = mime_guess::from_path(&asset.name).first_or_octet_stream();

    let mut header = ax::HeaderMap::new();
    header.insert(
        ax::header::CONTENT_TYPE,
        content_type.to_string().parse().unwrap(),
    );

    (ax::StatusCode::OK, header, asset.get_data(db).await)
}
