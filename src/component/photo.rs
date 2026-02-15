use std::hash::{Hash, Hasher};

use crate::database::SqliteError;
use crate::prelude::*;
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;

#[allow(dead_code)]
pub struct Photo {
    pub id: String,
    pub mark: bool,
    pub is_private: bool,
    pub source_path: String,
    pub source_time: i64,
}

impl Photo {
    pub fn setup(db: &Database) -> Result<(), Error> {
        db.execute_batch(
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
        .context("failed to create photos table")
    }

    fn from_row(row: &Row) -> Result<Self, SqliteError> {
        Ok(Self {
            id: row.get(0)?,
            mark: row.get(1)?,
            is_private: row.get(2)?,
            source_path: row.get(3)?,
            source_time: row.get(4)?,
        })
    }

    pub fn new(
        db: &Database,
        cfg: &Config,
        source_path: &Path,
        is_private: bool,
    ) -> Result<Photo, Error> {
        let source_time = source_path
            .metadata()?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        println!("loading photo {:?}", source_path);

        if let Ok(existing_photo) = Photo::get_by_path(db, source_path) {
            if existing_photo.source_time >= source_time {
                println!("photo is up to date, skipping");
                existing_photo.mark(db)?;
                return Ok(existing_photo);
            }

            println!("photo is outdated, updating");
            existing_photo.delete(db)?;
        } else {
            println!("photo is new, inserting");
        }

        let image_large = ImageReader::open(source_path)
            .context("failed to open photo")?
            .decode()
            .context("failed to decode photo")?;

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

        let mut data_large = vec![];
        let encoder_large = JpegEncoder::new_with_quality(&mut data_large, cfg.photo_quality);
        image_large
            .to_rgb8()
            .write_with_encoder(encoder_large)
            .context("failed to encode large photo")?;

        let mut data_small = vec![];
        let encoder_small = JpegEncoder::new_with_quality(&mut data_small, cfg.photo_quality);
        image_small
            .to_rgb8()
            .write_with_encoder(encoder_small)
            .context("failed to encode small photo")?;

        let source_path = source_path.to_str().unwrap();

        let mut hasher = std::hash::DefaultHasher::new();
        source_path.hash(&mut hasher);
        let id = format!("{:016x}", hasher.finish());

        db.query_one(
            r#"
                INSERT INTO photos (id, is_private, source_path, source_time, image_large_jpg, image_small_jpg)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING id, is_private, source_path, source_time, image_large_jpg, image_small_jpg
            "#,
            (id, is_private, source_path, source_time, data_large, data_small),
            Photo::from_row,
        ).context("failed to insert photo into database")
    }

    pub fn get_by_id(db: &Database, id: &str) -> Result<Photo, Error> {
        db.query_one(
            "SELECT id, mark, is_private, source_path, source_time FROM photos WHERE id = ?;",
            [id],
            |row| Self::from_row(row),
        )
        .context("failed to query photo by id from database")
    }

    pub fn get_by_path(db: &Database, source_path: &Path) -> Result<Photo, Error> {
        db.query_one(
            "SELECT id, mark, is_private, source_path, source_time FROM photos WHERE source_path = ?",
            [source_path.to_str().unwrap()],
            |row| Self::from_row(row),
        )
    }

    pub fn get_all(db: &Database, post_id: Option<&str>) -> Result<Vec<Photo>, Error> {
        let mut query = r#"
            SELECT photos.id, photos.mark, photos.is_private, photos.source_path, photos.source_time
            FROM photos
            JOIN posts_photos ON photos.id = posts_photos.photo_id
            JOIN posts ON posts_photos.post_id = posts.id
        "#
        .to_string();

        if post_id.is_some() {
            query.push_str("\nWHERE posts_photos.post_id = ?");
        }

        query.push_str("\nORDER BY posts.date DESC, photos.source_time DESC;");

        if let Some(post_id) = post_id {
            db.query_mul(&query, [post_id], |row| Self::from_row(row))
        } else {
            db.query_mul(&query, [], |row| Self::from_row(row))
        }
        .context("failed to query photos from database")
    }

    pub fn count_all(db: &Database) -> Result<u32, Error> {
        db.query_one("SELECT COUNT(*) FROM photos;", [], |row| row.get(0))
            .context("failed to count photos in database")
    }

    pub fn mark(&self, db: &Database) -> Result<(), Error> {
        db.execute("UPDATE photos SET mark = TRUE WHERE id = ?", [&self.id])
            .context("failed to mark photo in database")
    }

    pub fn delete(self, db: &Database) -> Result<(), Error> {
        db.execute("DELETE FROM photos WHERE id = ?", [&self.id])
            .context("failed to delete photo from database")
    }

    pub fn unmark_all(db: &Database) -> Result<(), Error> {
        db.execute("UPDATE photos SET mark = FALSE", [])
            .context("failed to unmark all photos in database")
    }

    pub fn delete_unmarked(db: &Database) -> Result<(), Error> {
        db.execute("DELETE FROM photos WHERE mark = FALSE", [])
            .context("failed to delete unmarked photos in database")
    }

    pub fn get_image_small(&self, db: &Database) -> Result<Vec<u8>, Error> {
        db.query_one(
            "SELECT image_small_jpg FROM photos WHERE id = ?;",
            [&self.id],
            |row| row.get(0),
        )
        .context("failed to query image_small from database")
    }

    pub fn get_image_large(&self, db: &Database) -> Result<Vec<u8>, Error> {
        db.query_one(
            "SELECT image_small_jpg FROM photos WHERE id = ?;",
            [&self.id],
            |row| row.get(0),
        )
        .context("failed to query image_large from database")
    }

    pub fn get_post(&self, db: &Database) -> Result<Post, Error> {
        db.query_one(
            "SELECT post_id FROM posts_photos WHERE photo_id = ?;",
            [&self.id],
            |row| row.get(0),
        )
        .and_then(|id: String| Post::by_id(db, &id))
        .context("failed to query post from database")
    }

    pub fn to_html(&self, link_url: &str, link_text: &str) -> PreEscaped<String> {
        html!(
            div class = "photo-preview" {
                div {
                    img class = "photo" src=(format!("/photos/{}?size=small", self.id)) alt = (format!("photo {}", self.id)) {}
                    a class = "photo-link" href = (link_url) { (link_text) }
                }
            }
        )
    }
}

pub async fn get_photos(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookies: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let cfg = &state.config.lock().unwrap();
    let user = User::from_cookie(db, &cookies).ok();

    let page = params
        .get("page")
        .map(|s| s.parse::<u32>().unwrap_or(1))
        .unwrap_or(1);

    println!("GET photos, page = {}, user = {:?}", page, user);

    let photos = match Photo::get_all(db, None) {
        Ok(photos) => photos
            .into_iter()
            .filter(|photo| !photo.is_private || user.is_some())
            .collect::<Vec<_>>(),
        Err(_) => return make_error(500, "Failed to get photos").into_response(),
    };

    let n_photos = photos.len() as u32;
    let last_page = n_photos / cfg.photos_per_page + u32::min(1, n_photos % cfg.photos_per_page);

    if page > last_page {
        return make_error(404, "Page not found").into_response();
    }

    let photos = photos
        .into_iter()
        .skip(((page - 1) * cfg.photos_per_page) as usize)
        .take(cfg.photos_per_page as usize);

    let content = html!(
        @for photo in photos {
            @let post = match photo.get_post(db) {
                Ok(post) => post,
                Err(_) => return make_error(500, "Failed to get post").into_response(),
            };

            (photo.to_html(&format!("/posts/{}/", post.id), "↪ to post"))
        }
        section id="photo-navigation" {
            @if page > 1 {
                a href="/photos/?page=1" { "<<first" } " "
                a href=(format!("/photos/?page={}", page - 1)) { "<prev" } " "
            }
            "page " (page) " of " (last_page)
            @if page < last_page {
                " " a href=(format!("/photos/?page={}", page + 1)) { "next>" }
                " " a href=(format!("/photos/?page={}", last_page)) { "last>>" }
            }
        }
    );

    let page = make_page(
        Some("Photos"),
        "A gallery of all photos.",
        vec!["/styles/photo.css"],
        content,
        user,
        false,
    );

    ax::Html::from(page.into_string()).into_response()
}

pub async fn get_photo(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(id): ax::Path<String>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let user = User::from_cookie(db, &cookie).ok();

    let size = match params.get("size").map(|s| s.as_str()) {
        Some("small") => "small",
        Some("large") => "large",
        _ => "large",
    };

    println!("GET photo {}, size = {}, user = {:?}", id, size, user);

    let photo = match Photo::get_by_id(db, &id) {
        Ok(photo) => photo,
        Err(_) => return make_error(404, "Photo not found").into_response(),
    };

    if photo.is_private && user.is_none() {
        return ax::StatusCode::FORBIDDEN.into_response();
    }

    let data = match match size {
        "small" => photo.get_image_small(db),
        "large" => photo.get_image_large(db),
        _ => unreachable!(),
    } {
        Ok(data) => data,
        Err(_) => return make_error(500, "Failed to get photo data").into_response(),
    };

    let header = ax::HeaderMap::from_iter(vec![(
        ax::header::CONTENT_TYPE,
        mime::IMAGE_JPEG.to_string().parse().unwrap(),
    )]);

    (header, data).into_response()
}
