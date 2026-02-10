use std::hash::{Hash, Hasher};

use crate::prelude::*;
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;

pub struct Photo {
    pub id: String,
    pub mark: bool,
    pub is_private: bool,
    pub source_path: String,
    pub source_time: i64,
}

impl Photo {
    pub async fn setup(db: &Database) {
        sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS photos (
                    id TEXT PRIMARY KEY,
                    mark BOOLEAN NOT NULL DEFAULT TRUE,
                    is_private BOOLEAN NOT NULL,
                    source_path TEXT NOT NULL UNIQUE,
                    source_time INTEGER NOT NULL,
                    image_large_jpg BLOB NOT NULL,
                    image_small_jpg BLOB NOT NULL
                );

                CREATE TABLE IF NOT EXISTS posts_photos (
                    post_id TEXT NOT NULL,
                    photo_id TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE,
                    FOREIGN KEY (photo_id) REFERENCES photos (id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS photos_id_index ON photos (id);
                CREATE INDEX IF NOT EXISTS photos_source_path_index ON photos (source_path);
            "#,
        )
        .execute(&db.pool)
        .await
        .expect("failed to create photos table");
    }

    pub async fn new(db: &Database, cfg: &Config, source_path: &Path, is_private: bool) -> Photo {
        let source_time = source_path
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        println!("loading photo {:?}", source_path);

        if let Some(existing_photo) = Photo::by_path(db, source_path).await {
            if existing_photo.source_time >= source_time {
                println!("photo is up to date, skipping");
                existing_photo.mark(db).await;
                return existing_photo;
            }

            println!("photo is outdated, updating");
            existing_photo.delete(db).await;
        } else {
            println!("photo is new, inserting");
        }

        let image_large = ImageReader::open(source_path)
            .expect("failed to open image")
            .decode()
            .expect("failed to decode image");

        println!("size: {}x{}", image_large.width(), image_large.height());

        let scale = f32::min(
            cfg.photo_max_preview_size as f32 / image_large.width() as f32,
            cfg.photo_max_preview_size as f32 / image_large.height() as f32,
        );

        let image_small = image_large.resize(
            (image_large.width() as f32 * scale) as u32,
            (image_large.height() as f32 * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        );

        let mut data_large = Vec::new();
        let encoder_large = JpegEncoder::new_with_quality(&mut data_large, cfg.photo_quality);
        image_large
            .to_rgb8()
            .write_with_encoder(encoder_large)
            .expect("failed to encode large image as JPEG");

        let mut data_small = Vec::new();
        let encoder_small = JpegEncoder::new_with_quality(&mut data_small, cfg.photo_quality);
        image_small
            .to_rgb8()
            .write_with_encoder(encoder_small)
            .expect("failed to encode small image as JPEG");

        let source_path = source_path.to_str().unwrap();

        let mut hasher = std::hash::DefaultHasher::new();
        source_path.hash(&mut hasher);
        let id = format!("{:016x}", hasher.finish());

        sqlx::query(
            r#"
                    INSERT INTO photos (id, is_private, source_path, source_time, image_large_jpg, image_small_jpg)
                    VALUES (?, ?, ?, ?, ?, ?)
                    RETURNING id
                "#
        )
        .bind(&id)
        .bind(is_private)
        .bind(source_path)
        .bind(source_time)
        .bind(data_large)
        .bind(data_small)
        .execute(&db.pool)
        .await
        .expect("failed to insert photo into database");

        Self::by_id(db, &id).await.unwrap()
    }

    pub async fn by_id(db: &Database, id: &str) -> Option<Photo> {
        sqlx::query(
            r#"
                SELECT id, mark, is_private, source_path, source_time
                FROM photos
                WHERE id = ?;
            "#,
        )
        .bind(id)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query photo by source path from database")
        .map(|row| Photo {
            id: row.get(0),
            mark: row.get(1),
            is_private: row.get(2),
            source_path: row.get(3),
            source_time: row.get(4),
        })
    }

    pub async fn by_path(db: &Database, source_path: &Path) -> Option<Photo> {
        let source_path = source_path.to_str().unwrap();

        sqlx::query(
            r#"
                SELECT id, mark, is_private, source_path, source_time
                FROM photos
                WHERE source_path = ?
            "#,
        )
        .bind(source_path)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query photo by source path from database")
        .map(|row| Photo {
            id: row.get(0),
            mark: row.get(1),
            is_private: row.get(2),
            source_path: row.get(3),
            source_time: row.get(4),
        })
    }

    pub async fn list(db: &Database, post_id: Option<&str>) -> Vec<Photo> {
        if let Some(post_id) = post_id {
            sqlx::query(
                r#"
                    SELECT photos.id, photos.mark, photos.is_private, photos.source_path, photos.source_time
                    FROM photos
                    JOIN posts_photos ON photos.id = posts_photos.photo_id
                    WHERE posts_photos.post_id = ?;
                "#,
            )
            .bind(post_id)
        } else {
            sqlx::query("SELECT id, mark, is_private, source_path, source_time FROM photos")
        }
        .fetch_all(&db.pool)
        .await
        .expect("failed to query photos from database")
        .into_iter()
        .map(|row| Photo {
            id: row.get(0),
            mark: row.get(1),
            is_private: row.get(2),
            source_path: row.get(3),
            source_time: row.get(4),
        })
        .collect::<Vec<_>>()
    }

    pub async fn mark(&self, db: &Database) {
        sqlx::query("UPDATE photos SET mark = TRUE WHERE id = ?")
            .bind(&self.id)
            .execute(&db.pool)
            .await
            .expect("failed to mark photo in database");
    }

    pub async fn delete(self, db: &Database) {
        sqlx::query("DELETE FROM photos WHERE id = ?")
            .bind(self.id)
            .execute(&db.pool)
            .await
            .expect("failed to delete photo from database");
    }

    pub async fn unmark_all(db: &Database) {
        sqlx::query("UPDATE photos SET mark = FALSE")
            .execute(&db.pool)
            .await
            .expect("failed to unmark all photos in database");
    }

    pub async fn delete_unmarked(db: &Database) {
        sqlx::query("DELETE FROM photos WHERE mark = FALSE")
            .execute(&db.pool)
            .await
            .expect("failed to delete unmarked photos in database");
    }

    pub async fn get_image_small(&self, db: &Database) -> Vec<u8> {
        sqlx::query("SELECT image_small_jpg FROM photos WHERE id = ?;")
            .bind(&self.id)
            .fetch_one(&db.pool)
            .await
            .expect("failed to query image_small_jpg from database")
            .get(0)
    }

    pub async fn get_image_large(&self, db: &Database) -> Vec<u8> {
        sqlx::query("SELECT image_large_jpg FROM photos WHERE id = ?;")
            .bind(&self.id)
            .fetch_one(&db.pool)
            .await
            .expect("failed to query image_large_jpg from database")
            .get(0)
    }
}

pub async fn get_photo(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(id): ax::Path<String>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
) -> (ax::StatusCode, ax::HeaderMap, Vec<u8>) {
    let db = &state.db;

    let size = match params.get("size").map(|s| s.as_str()) {
        Some("small") => "small",
        Some("large") => "large",
        _ => "small",
    };

    println!("GET photo {}, size = {}", id, size);

    let photo = match Photo::by_id(db, &id).await {
        Some(photo) => photo,
        None => return (ax::StatusCode::NOT_FOUND, ax::HeaderMap::new(), Vec::new()),
    };

    let data = match size {
        "small" => photo.get_image_small(db).await,
        "large" => photo.get_image_large(db).await,
        _ => unreachable!(),
    };

    let mut header = ax::HeaderMap::new();
    header.insert(
        ax::header::CONTENT_TYPE,
        mime::IMAGE_JPEG.to_string().parse().unwrap(),
    );

    (ax::StatusCode::OK, header, data)
}
