use crate::prelude::*;
use std::hash::{Hash, Hasher};

#[allow(dead_code)]
pub struct User {
    pub key_hash: String,
    pub group_name: String,
}

impl User {
    pub async fn setup(db: &Database) {
        sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS users (
                    key_hash TEXT PRIMARY KEY,
                    group_name TEXT NOT NULL
                );
            "#,
        )
        .execute(&db.pool)
        .await
        .expect("failed to create users table");
    }

    fn from_row(row: sqlx::sqlite::SqliteRow) -> Self {
        Self {
            key_hash: row.get(0),
            group_name: row.get(1),
        }
    }

    pub async fn new(db: &Database, key_hash: &str, group_name: &str) -> Self {
        let key_hash = Self::key_hash(key_hash);

        sqlx::query(
            r#"
                INSERT INTO users (key_hash, group_name) VALUES (?, ?)
            "#,
        )
        .bind(&key_hash)
        .bind(group_name)
        .execute(&db.pool)
        .await
        .expect("failed to insert user into database");

        Self {
            key_hash,
            group_name: group_name.to_string(),
        }
    }

    pub async fn from_cookie(db: &Database, cookies: &ax::CookieJar) -> Option<User> {
        let key = cookies.get("key").map(|key| key.value().to_string());

        if let Some(key) = key {
            Self::by_hash(db, &key).await
        } else {
            None
        }
    }

    pub async fn by_hash(db: &Database, key_hash: &str) -> Option<User> {
        sqlx::query(
            r#"
                SELECT key_hash, group_name FROM users WHERE key_hash = ?;
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&db.pool)
        .await
        .expect("failed to query user by password from database")
        .map(User::from_row)
    }

    pub async fn delete_all(db: &Database) {
        sqlx::query("DELETE FROM users")
            .execute(&db.pool)
            .await
            .expect("failed to delete all users from database");
    }

    fn key_hash(key: &str) -> String {
        let mut hasher = std::hash::DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.group_name)
    }
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User(\"{}\")", self.group_name)
    }
}

pub async fn get_login(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db;
    let user = User::from_cookie(db, &cookie).await;
    let failed = if let Some(failed) = params.get("failed") {
        failed == "true"
    } else {
        false
    };

    println!("GET login, failed = {}, user = {:?}", failed, user);

    let content = html!(
        @if failed {
            p { "Invalid password, please try again." }
        }

        form action="/login/" method="post" {
            input type="password" name="key" placeholder="password" required {}
            input type="submit" value="Login" {}
        }
    );

    let page = make_page(
        Some("Login"),
        "Login page.",
        vec!["/styles/login.css"],
        content,
        user,
    );

    ax::Html::from(page.into_string()).into_response()
}

#[derive(Deserialize, Debug)]
pub struct LoginForm {
    key: String,
}

pub async fn post_login(
    ax::State(state): ax::State<Arc<AppState>>,
    form: ax::Form<LoginForm>,
) -> impl IntoResponse {
    let db = &state.db;

    let hash = User::key_hash(&form.key);
    let user = User::by_hash(db, &hash).await;

    if let Some(user) = user {
        println!("POST login, user = {:?}", user);
        (
            ax::CookieJar::new().add(ax::Cookie::build(("key", hash)).path("/")),
            ax::Redirect::to("/"),
        )
            .into_response()
    } else {
        println!("POST login, invalid key");
        ax::Redirect::to("/login/?failed=true").into_response()
    }
}

pub async fn post_logout(cookie: ax::CookieJar) -> impl IntoResponse {
    println!("POST logout");
    (
        cookie.add(ax::Cookie::build("key").path("/").removal().build()),
        ax::Redirect::to("/"),
    )
        .into_response()
}
