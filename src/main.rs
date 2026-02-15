mod component;
mod config;
mod database;
mod error;
mod prelude;
mod state;

use crate::prelude::*;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    match args.get(1).map(|s| s.as_str()) {
        Some("build") => build().await.unwrap(),
        Some("serve") => serve().await.unwrap(),
        _ => {
            eprintln!("Usage: {} [build|serve]", args[0]);
            std::process::exit(1);
        }
    }
}

async fn build() -> Result<(), Error> {
    let config = Config::from_json_file("website.json")?;
    let db = Database::connect(&config.database_path)?;

    Post::setup(&db)?;
    Asset::setup(&db)?;
    Photo::setup(&db)?;
    File::setup(&db)?;
    User::setup(&db)?;

    Post::delete_all(&db)?;
    Photo::unmark_all(&db)?;
    File::delete_all(&db)?;
    Asset::delete_all(&db)?;
    User::delete_all(&db)?;

    for user in &config.users {
        User::new(&db, &user.key, &user.group)?;
    }

    for parent in fs::read_dir(&config.files_path).expect("failed to read files directory") {
        let parent = parent?;
        for entry in fs::read_dir(parent.path()).expect("failed to read files directory") {
            File::new(&db, &parent.path(), &entry?.path())?;
        }
    }

    for post_path in fs::read_dir(&config.posts_path).expect("failed to read posts directory") {
        Post::new(&db, &config, &post_path?.path())?;
    }

    Photo::delete_unmarked(&db)?;

    println!("all done!");

    Ok(())
}

async fn serve() -> Result<(), Error> {
    let config = Config::from_json_file("website.json")?;
    let db = Database::connect(&config.database_path)?;

    let state = Arc::new(AppState {
        db: Arc::new(Mutex::new(db)),
        config: Arc::new(Mutex::new(config.clone())),
    });

    let app = ax::Router::new()
        .route("/", ax::routing::get(get_index))
        .route("/posts/", ax::routing::get(get_posts))
        .route("/posts/{id}/", ax::routing::get(get_post))
        .route("/posts/{id}/assets/{name}", ax::routing::get(get_asset))
        .route("/photos/", ax::routing::get(get_photos))
        .route("/photos/{id}", ax::routing::get(get_photo))
        .route("/projects/", ax::routing::get(get_projects))
        .route("/files/{name}", ax::routing::get(get_file_file))
        .route("/styles/{name}", ax::routing::get(get_file_style))
        .route("/assets/{name}", ax::routing::get(get_file_asset))
        .route("/login/", ax::routing::get(get_login))
        .route("/login/", ax::routing::post(post_login))
        .route("/logout/", ax::routing::post(post_logout))
        .fallback(ax::routing::get(get_not_found))
        .with_state(state);

    let listener = TcpListener::bind(format!("{}:{}", config.server_host, config.server_port))
        .await
        .context("failed to bind server")?;

    println!(
        "Server running on http://{}:{}",
        config.server_host, config.server_port
    );

    axum::serve(listener, app)
        .await
        .context("failed to start server")?;

    Ok(())
}

// fn make_redirect(path: &str) -> axum::routing::MethodRouter<Arc<AppState>> {
//     ax::routing::get(async || ax::Redirect::to("/photos/"))
// }
