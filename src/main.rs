mod component;
mod config;
mod database;
mod prelude;
mod state;

use crate::prelude::*;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    match args.get(1).map(|s| s.as_str()) {
        Some("build") => build().await,
        Some("serve") => serve().await,
        _ => {
            eprintln!("Usage: {} [build|serve]", args[0]);
            std::process::exit(1);
        }
    }
}

async fn build() {
    let config = Config::from_json_file("website.json");
    let db = Database::connect(&config.database_path).await;

    Post::setup(&db).await;
    Asset::setup(&db).await;
    Photo::setup(&db).await;
    File::setup(&db).await;

    Post::delete_all(&db).await;
    Photo::unmark_all(&db).await;
    File::delete_all(&db).await;
    Asset::delete_all(&db).await;

    for parent in fs::read_dir(&config.files_path).expect("failed to read files directory") {
        let parent = parent.unwrap();
        for entry in fs::read_dir(parent.path()).expect("failed to read files directory") {
            File::new(&db, &parent.path(), &entry.unwrap().path()).await;
        }
    }

    for post_path in fs::read_dir(&config.posts_path).expect("failed to read posts directory") {
        Post::new(&db, &config, &post_path.unwrap().path()).await;
    }

    Photo::delete_unmarked(&db).await;

    println!("all done!");
}

async fn serve() {
    let config = Config::from_json_file("website.json");
    let db = Database::connect(&config.database_path).await;

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
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
        .fallback(ax::routing::get(get_error))
        .with_state(state);

    let listener = TcpListener::bind(format!("{}:{}", config.server_host, config.server_port))
        .await
        .expect("failed to bind server");

    println!(
        "Server running on http://{}:{}",
        config.server_host, config.server_port
    );

    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}
