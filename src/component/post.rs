use crate::component::error::make_error;
use crate::prelude::*;
use rand::random;

#[derive(Serialize, Deserialize)]
struct PostMetadata {
    pub id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub date: String,
    pub tags: Vec<String>,
    pub permalink: Option<String>,
}

impl PostMetadata {
    fn from_json_str(json_str: &str) -> PostMetadata {
        serde_json::from_str(json_str).expect("failed to decode post metadata")
    }

    fn from_json_file(path: &str) -> PostMetadata {
        let json_str = fs::read_to_string(path).expect("failed to read metadata file");
        PostMetadata::from_json_str(&json_str)
    }

    fn to_json_str(&self) -> String {
        let mut buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        self.serialize(&mut ser)
            .expect("failed to serialize post metadata");
        String::from_utf8(buf).unwrap()
    }

    fn to_json_file(&self, path: &str) {
        fs::write(path, self.to_json_str()).expect("failed to write metadata file");
    }
}

pub struct Post {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub date: String,
    pub permalink: Option<String>,
}

impl Post {
    pub async fn setup(db: &Database) {
        sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS posts (
                    id TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    description TEXT NULL,
                    date TEXT NOT NULL,
                    permalink TEXT NULL,
                    source TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS posts_id_index ON posts (id);

                CREATE TABLE IF NOT EXISTS posts_tags (
                    post_id TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE
                );
            "#,
        )
        .execute(&db.pool)
        .await
        .expect("failed to create posts table");
    }

    pub async fn new(db: &Database, cfg: &Config, source_path: &Path) -> Post {
        println!("loading post {:?}", source_path);

        let index_path = source_path.join(&cfg.post_content_path);
        let metadata_path = source_path.join(&cfg.post_metadata_path);

        let source = fs::read_to_string(&index_path).expect("failed to read post content file");
        let mut metadata = PostMetadata::from_json_file(metadata_path.to_str().unwrap());

        if metadata.id.is_none() {
            let id: u64 = random();
            metadata.id = Some(format!("{:016x}", id));
            metadata.to_json_file(metadata_path.to_str().unwrap());
        }

        println!("id: {}", metadata.id.as_ref().unwrap());
        println!("title: {}", metadata.title);
        println!("date: {}", metadata.date);
        println!("tags: {:?}", metadata.tags);

        sqlx::query(
            r#"
                INSERT INTO posts (id, title, description, date, permalink, source)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING id;
            "#,
        )
        .bind(metadata.id.as_ref().unwrap())
        .bind(&metadata.title)
        .bind(&metadata.description)
        .bind(&metadata.date)
        .bind(&metadata.permalink)
        .bind(&source)
        .execute(&db.pool)
        .await
        .expect("failed to insert post into database");

        let assets_path = source_path.join(&cfg.post_assets_path);
        let public_photos_path = source_path.join(&cfg.post_public_photos_path);
        let private_photos_path = source_path.join(&cfg.post_private_photos_path);

        if assets_path.exists() {
            for asset_path in fs::read_dir(assets_path).expect("failed to read assets directory") {
                let asset = Asset::new(db, &asset_path.unwrap().path()).await;
                sqlx::query("INSERT INTO posts_assets (post_id, asset_id) VALUES (?, ?);")
                    .bind(metadata.id.as_ref().unwrap())
                    .bind(asset.id)
                    .execute(&db.pool)
                    .await
                    .expect("failed to insert into posts_assets table");
            }
        }

        if let Ok(public_photos) = fs::read_dir(&public_photos_path) {
            for photo_path in public_photos {
                let photo = Photo::new(db, cfg, &photo_path.unwrap().path(), false).await;
                sqlx::query("INSERT INTO posts_photos (post_id, photo_id) VALUES (?, ?);")
                    .bind(metadata.id.as_ref().unwrap())
                    .bind(photo.id)
                    .execute(&db.pool)
                    .await
                    .expect("failed to insert into posts_photos table");
            }
        }

        if let Ok(private_photos) = fs::read_dir(&private_photos_path) {
            for photo_path in private_photos {
                let photo = Photo::new(db, cfg, &photo_path.unwrap().path(), true).await;
                sqlx::query("INSERT INTO posts_photos (post_id, photo_id) VALUES (?, ?);")
                    .bind(metadata.id.as_ref().unwrap())
                    .bind(photo.id)
                    .execute(&db.pool)
                    .await
                    .expect("failed to insert into posts_photos table");
            }
        }

        let post = Self::by_id(db, metadata.id.as_ref().unwrap())
            .await
            .unwrap();
        post.set_tags(db, &metadata.tags).await;
        post
    }

    pub async fn by_id(db: &Database, id: &str) -> Option<Post> {
        sqlx::query(
            r#"
                SELECT id, title, description, date, permalink
                FROM posts
                WHERE id = ?;
            "#,
        )
        .bind(id)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query photo by source path from database")
        .map(|row| Post {
            id: row.get(0),
            title: row.get(1),
            description: row.get(2),
            date: row.get(3),
            permalink: row.get(4),
        })
    }

    pub async fn delete_all(db: &Database) {
        sqlx::query("DELETE FROM posts")
            .execute(&db.pool)
            .await
            .expect("failed to delete all posts from database");
    }

    pub async fn set_tags(&self, db: &Database, tags: &[String]) {
        sqlx::query("DELETE FROM posts_tags WHERE post_id = ?")
            .bind(&self.id)
            .execute(&db.pool)
            .await
            .expect("failed to delete existing tags from database");

        for tag in tags {
            sqlx::query("INSERT INTO posts_tags (post_id, tag) VALUES (?, ?); ")
                .bind(&self.id)
                .bind(tag)
                .execute(&db.pool)
                .await
                .expect("failed to insert tag into posts_tags table");
        }
    }

    pub async fn get_tags(&self, db: &Database) -> Vec<String> {
        sqlx::query("SELECT tag FROM posts_tags WHERE post_id = ?;")
            .bind(&self.id)
            .fetch_all(&db.pool)
            .await
            .expect("failed to query tags for post from database")
            .iter()
            .map(|row| row.get(0))
            .collect()
    }

    pub async fn get_source(&self, db: &Database) -> String {
        sqlx::query("SELECT source FROM posts WHERE id = ?;")
            .bind(&self.id)
            .fetch_one(&db.pool)
            .await
            .expect("failed to query source for post from database")
            .get(0)
    }

    pub async fn list(db: &Database, limit: Option<u32>) -> Vec<Post> {
        let limit = limit.unwrap_or(10000);

        let rows = sqlx::query(
            r#"
                SELECT id, title, description, date, permalink
                FROM posts
                ORDER BY date DESC
                LIMIT ?;
            "#,
        )
        .bind(limit)
        .fetch_all(&db.pool)
        .await
        .expect("failed to query posts from database");

        let mut posts = Vec::new();

        for row in rows {
            posts.push(Post {
                id: row.get(0),
                title: row.get(1),
                description: row.get(2),
                date: row.get(3),
                permalink: row.get(4),
            });
        }

        posts
    }
}

pub async fn get_post(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(id): ax::Path<String>,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let db = &state.db;

    println!("GET post {}", id);

    let post = match Post::by_id(db, &id).await {
        Some(post) => post,
        None => {
            return make_error(404, "Failed to find post.");
        }
    };

    let mut content = markdown_to_html(&post.get_source(db).await);
    content.push_str(
        &html! {
            @for photo in Photo::list(db, Some(&post.id)).await {
                img src=(format!("/photos/{}", photo.id)) alt=(format!("photo {}", photo.id)) {}
            }
        }
        .into_string(),
    );

    let page = make_page(
        &post.title,
        &post.description.unwrap_or("".to_string()),
        &content,
    );

    (
        ax::StatusCode::OK,
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}

fn markdown_to_html(markdown: &str) -> String {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let mut content = String::new();
    comrak::format_html(root, &comrak::Options::default(), &mut content).unwrap();
    content
}
