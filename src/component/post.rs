use crate::prelude::*;

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
        let mut buf = vec![];
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

#[allow(dead_code)]
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

    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get(0),
            title: row.get(1),
            description: row.get(2),
            date: row.get(3),
            permalink: row.get(4),
        }
    }

    pub async fn new(db: &Database, cfg: &Config, source_path: &Path) -> Post {
        println!("loading post {:?}", source_path);

        let index_path = source_path.join(&cfg.post_content_path);
        let metadata_path = source_path.join(&cfg.post_metadata_path);

        let source = fs::read_to_string(&index_path).expect("failed to read post content file");
        let mut metadata = PostMetadata::from_json_file(metadata_path.to_str().unwrap());

        if metadata.id.is_none() {
            let id: u64 = rand::random();
            metadata.id = Some(format!("{:016x}", id));
            metadata.to_json_file(metadata_path.to_str().unwrap());
        }

        metadata.tags = metadata
            .tags
            .iter()
            .map(|tag| tag.to_lowercase().replace(" ", "_"))
            .collect();

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
            for asset_path in fs::read_dir(assets_path).expect("failed to read styles directory") {
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
        sqlx::query("SELECT id, title, description, date, permalink FROM posts WHERE id = ?;")
            .bind(id)
            .fetch_optional(&db.pool)
            .await
            .expect("failed to query photo by source path from database")
            .as_ref()
            .map(Post::from_row)
    }

    pub async fn by_permalink(db: &Database, permalink: &str) -> Option<Post> {
        sqlx::query(
            r#"
                SELECT id, title, description, date, permalink
                FROM posts WHERE permalink = ?;
            "#,
        )
        .bind(permalink)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query post id by permalink from database")
        .as_ref()
        .map(Post::from_row)
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

        sqlx::query(
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
        .expect("failed to query posts from database")
        .iter()
        .map(Post::from_row)
        .collect()
    }
}

pub async fn get_post(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(id): ax::Path<String>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db;
    let user = User::from_cookie(db, &cookie).await;

    println!("GET post {}, user = {:?}", id, user);

    let post = match Post::by_id(db, &id).await {
        Some(post) => post,
        None => {
            return match Post::by_permalink(db, &id).await {
                Some(post) => ax::Redirect::to(&format!("/posts/{}/", post.id)).into_response(),
                None => make_error(404, "Failed to find post.", user).into_response(),
            };
        }
    };

    let tags = post.get_tags(db).await;
    let (photos, n_hidden) = Photo::list(db, user.is_some(), Some(&post.id), None, None).await;

    let content = html!(
        section class="post-info" {
            p { (post.date) }
            p {
                @for tag in tags {
                    a class="tag" href=(format!("/posts/?tag={}", tag)) { code { (format!("#{}", tag)) } } " ";
                }
            }
        }

        br{}

        (PreEscaped(markdown_to_html(&post.get_source(db).await)))

        @for photo in photos {
            (photo.to_html(&format!("/photos/{}?size=large/", photo.id), "↪ full res").await)
        }

        @if n_hidden > 0 {
            p id="hidden-message" { "(" (n_hidden) " photos hidden, " a href="/login/" { "log in" } " to see all)" }
        }
    );

    let page = make_page(
        Some(&post.title),
        &post.description.unwrap_or("".to_string()),
        vec!["/styles/photo.css", "/styles/post.css"],
        content,
        user,
    );

    ax::Html::from(page.into_string()).into_response()
}

pub async fn get_posts(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db;
    let tag = params.get("tag").map(|s| s.to_lowercase());
    let user = User::from_cookie(db, &cookie).await;

    println!("GET posts, tag: {:?}, user = {:?}", tag, user);

    let content = html! {
        @if let Some(tag) = tag.as_ref() {
            section class="post-header" {
                p { "Only showing posts tagged with " a class="tag" href=(format!("/posts/?tag={}", tag)) { code { (format!("#{}", tag)) } } }
                p { a href="/posts/" { "> show all <" } }
            }
        }

        (make_posts_table(db, tag, None, false, true).await)
    };

    let page = make_page(
        Some("Posts"),
        "A list of all posts.",
        vec!["/styles/post.css"],
        content,
        user,
    );

    ax::Html::from(page.into_string()).into_response()
}

pub async fn make_posts_table(
    db: &Database,
    tag: Option<String>,
    limit: Option<u32>,
    with_description: bool,
    with_date: bool,
) -> PreEscaped<String> {
    let posts = Post::list(db, limit).await;

    html!(
        table class="post-table" {
            @for post in posts {
                @let tags = post.get_tags(db).await;

                @if tag.is_none() || tags.contains(tag.as_ref().unwrap()) {
                    tr {
                        td {
                            div class="post-title" {
                                a href=(format!("/posts/{}/", post.id))  { (post.title) }
                            }
                            div class="post-tags" {
                                @for tag in tags {
                                    a class="tag" href=(format!("/posts/?tag={}", tag)) { code { (format!("#{}", tag)) } } " ";
                                }
                            }
                            @if with_description {
                                div class="post-description" { (post.description.unwrap_or("".to_string())) }
                            }
                        }
                        @if with_date {
                            td class="post-date" { (post.date) }
                        }
                    }
                }
            }
        }
    )
}

fn markdown_to_html(markdown: &str) -> String {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let mut content = String::new();
    comrak::format_html(root, &comrak::Options::default(), &mut content).unwrap();
    content
}

// fn next_color(prev_color: &mut Option<u32>) -> u32 {
//     loop {
//         let color = (rand::random::<u32>() % 10) + 1;
//         if prev_color.is_none() || color != prev_color.unwrap() {
//             *prev_color = Some(color);
//             return color;
//         }
//     }
// }
