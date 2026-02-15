use crate::database::SqliteError;
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
    fn from_json_str(json_str: &str) -> Result<PostMetadata, Error> {
        serde_json::from_str(json_str).context("failed to decode post metadata")
    }

    fn from_json_file(path: &str) -> Result<PostMetadata, Error> {
        let json_str = fs::read_to_string(path).context("failed to read metadata file")?;
        PostMetadata::from_json_str(&json_str)
    }

    fn to_json_str(&self) -> Result<String, Error> {
        let mut buf = vec![];
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        self.serialize(&mut ser)
            .context("failed to serialize post metadata")?;
        Ok(String::from_utf8(buf)?)
    }

    fn to_json_file(&self, path: &str) -> Result<(), Error> {
        fs::write(path, self.to_json_str()?).context("failed to write metadata file")
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
    pub fn setup(db: &Database) -> Result<(), Error> {
        db.execute_batch(
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
        .context("failed to create posts table")
    }

    fn from_row(row: &Row) -> Result<Self, SqliteError> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            date: row.get(3)?,
            permalink: row.get(4)?,
        })
    }

    pub fn new(db: &Database, cfg: &Config, source_path: &Path) -> Result<Post, Error> {
        println!("loading post {:?}", source_path);

        let index_path = source_path.join(&cfg.post_content_path);
        let metadata_path = source_path.join(&cfg.post_metadata_path);

        let source = fs::read_to_string(&index_path).context("failed to read post content file")?;
        let mut metadata = PostMetadata::from_json_file(metadata_path.to_str().unwrap())?;

        if metadata.id.is_none() {
            let id: u64 = rand::random();
            metadata.id = Some(format!("{:016x}", id));
            metadata.to_json_file(metadata_path.to_str().unwrap())?;
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

        let post = db
            .query_one(
                r#"
                INSERT INTO posts (id, title, description, date, permalink, source)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING id, title, description, date, permalink, source;
            "#,
                (
                    metadata.id.as_ref().unwrap(),
                    &metadata.title,
                    &metadata.description,
                    &metadata.date,
                    &metadata.permalink,
                    &source,
                ),
                Post::from_row,
            )
            .context("failed to insert post into database")?;

        let assets_path = source_path.join(&cfg.post_assets_path);
        let public_photos_path = source_path.join(&cfg.post_public_photos_path);
        let private_photos_path = source_path.join(&cfg.post_private_photos_path);

        if assets_path.exists() {
            for asset_path in fs::read_dir(assets_path).expect("failed to read styles directory") {
                let asset = Asset::new(db, &asset_path?.path())?;
                db.execute(
                    "INSERT INTO posts_assets (post_id, asset_id) VALUES (?, ?);",
                    (metadata.id.as_ref().unwrap(), asset.id),
                )
                .context("failed to insert into posts_assets table")?;
            }
        }

        if let Ok(public_photos) = fs::read_dir(&public_photos_path) {
            for photo_path in public_photos {
                let photo = Photo::new(db, cfg, &photo_path?.path(), false)?;
                db.execute(
                    "INSERT INTO posts_photos (post_id, photo_id) VALUES (?, ?);",
                    (metadata.id.as_ref().unwrap(), photo.id),
                )
                .context("failed to insert into posts_photos table")?;
            }
        }

        if let Ok(private_photos) = fs::read_dir(&private_photos_path) {
            for photo_path in private_photos {
                let photo = Photo::new(db, cfg, &photo_path?.path(), true)?;
                db.execute(
                    "INSERT INTO posts_photos (post_id, photo_id) VALUES (?, ?);",
                    (metadata.id.as_ref().unwrap(), photo.id),
                )
                .context("failed to insert into posts_photos table")?;
            }
        }

        post.set_tags(db, &metadata.tags)?;
        Ok(post)
    }

    pub fn by_id(db: &Database, id: &str) -> Result<Post, Error> {
        db.query_one(
            "SELECT id, title, description, date, permalink FROM posts WHERE id = ?;",
            [id],
            Post::from_row,
        )
        .context("failed to query photo by source path from database")
    }

    pub fn by_permalink(db: &Database, permalink: &str) -> Result<Post, Error> {
        db.query_one(
            "SELECT id, title, description, date, permalink FROM posts WHERE permalink = ?;",
            [permalink],
            Post::from_row,
        )
        .context("failed to query post id by permalink from database")
    }

    pub fn delete_all(db: &Database) -> Result<(), Error> {
        db.execute("DELETE FROM posts", [])
            .context("failed to delete all posts from database")
    }

    pub fn set_tags(&self, db: &Database, tags: &[String]) -> Result<(), Error> {
        db.execute("DELETE FROM posts_tags WHERE post_id = ?", [&self.id])
            .context("failed to delete existing tags from database")?;

        for tag in tags {
            db.execute(
                "INSERT INTO posts_tags (post_id, tag) VALUES (?, ?);",
                (&self.id, tag),
            )
            .context("failed to insert tag into posts_tags table")?;
        }

        Ok(())
    }

    pub fn get_tags(&self, db: &Database) -> Result<Vec<String>, Error> {
        db.query_mul(
            "SELECT tag FROM posts_tags WHERE post_id = ?;",
            [&self.id],
            |row| row.get(0),
        )
        .context("failed to query tags for post from database")
    }

    pub fn get_source(&self, db: &Database) -> Result<String, Error> {
        db.query_one(
            "SELECT source FROM posts WHERE id = ?;",
            [&self.id],
            |row| row.get(0),
        )
        .context("failed to query source for post from database")
    }

    pub fn get_all(db: &Database) -> Result<Vec<Post>, Error> {
        db.query_mul(
            r#"
                SELECT id, title, description, date, permalink
                FROM posts
                ORDER BY date DESC;
            "#,
            [],
            Post::from_row,
        )
        .context("failed to query posts from database")
    }
}

pub async fn get_post(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Path(id): ax::Path<String>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let user = User::from_cookie(db, &cookie).ok();

    println!("GET post {}, user = {:?}", id, user);

    let post = match Post::by_id(db, &id) {
        Ok(post) => post,
        Err(_) => {
            return match Post::by_permalink(db, &id) {
                Ok(post) => ax::Redirect::to(&format!("/posts/{}/", post.id)).into_response(),
                Err(_) => make_error(404, "Post not found").into_response(),
            };
        }
    };

    let tags = match post.get_tags(db) {
        Ok(tags) => tags,
        Err(_) => return make_error(500, "Failed to load tags").into_response(),
    };

    let photos_all = match Photo::get_all(db, Some(&post.id)) {
        Ok(photos) => photos,
        Err(_) => return make_error(500, "Failed to load photos").into_response(),
    };

    let photos_filtered: Vec<_> = photos_all
        .iter()
        .filter(|photo| !photo.is_private || user.is_some())
        .collect();

    let n_hidden = photos_all.len() - photos_filtered.len();

    let source_md = match post.get_source(db) {
        Ok(source_md) => source_md,
        Err(_) => return make_error(500, "Failed to load markdown").into_response(),
    };

    let source_html = match markdown_to_html(&source_md) {
        Ok(source_html) => source_html,
        Err(_) => return make_error(500, "Failed to get html").into_response(),
    };

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

        (PreEscaped(source_html))

        @for photo in photos_filtered {
            (photo.to_html(&format!("/photos/{}?size=large/", photo.id), "↪ full res"))
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
        false,
    );

    ax::Html::from(page.into_string()).into_response()
}

pub async fn get_posts(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let tag = params.get("tag").map(|s| s.to_lowercase());
    let user = User::from_cookie(db, &cookie).ok();

    println!("GET posts, tag: {:?}, user = {:?}", tag, user);

    let posts_table = match make_posts_table(db, tag.clone(), None, false, true) {
        Ok(posts_table) => posts_table,
        Err(_) => return make_error(500, "Failed to load posts table").into_response(),
    };

    let content = html! {
        @if let Some(tag) = tag.as_ref() {
            section class="post-header" {
                p { "Only showing posts tagged with " a class="tag" href=(format!("/posts/?tag={}", tag)) { code { (format!("#{}", tag)) } } }
                p { a href="/posts/" { "> show all <" } }
            }
        }

        (posts_table)
    };

    let page = make_page(
        Some("Posts"),
        "A list of all posts.",
        vec!["/styles/post.css"],
        content,
        user,
        false,
    );

    ax::Html::from(page.into_string()).into_response()
}

pub fn make_posts_table(
    db: &Database,
    tag: Option<String>,
    limit: Option<u32>,
    with_description: bool,
    with_date: bool,
) -> Result<PreEscaped<String>, Error> {
    let posts = Post::get_all(db)?
        .into_iter()
        .take(limit.unwrap_or(u32::MAX) as usize)
        .collect::<Vec<_>>();

    Ok(html!(
        table class="post-table" {
            @for post in posts {
                @let tags = post.get_tags(db)?;

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
    ))
}

fn markdown_to_html(markdown: &str) -> Result<String, Error> {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());
    let mut content = String::new();
    comrak::format_html(root, &comrak::Options::default(), &mut content)
        .context("failed to compile markdown")?;
    Ok(content)
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
