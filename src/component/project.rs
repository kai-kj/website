use crate::prelude::*;

pub async fn get_projects(
    ax::State(state): ax::State<Arc<AppState>>,
    cookie: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let user = User::from_cookie(db, &cookie).ok();

    println!("GET projects, user = {:?}", user);

    let posts_table = match make_posts_table(db, Some("project".to_string()), None, true, false) {
        Ok(posts_table) => posts_table,
        Err(_) => return make_error(500, "Failed to load posts table").into_response(),
    };

    let page = make_page(
        Some("Projects"),
        "A list of all projects.",
        vec!["/styles/post.css"],
        posts_table,
        user,
        false,
    );

    ax::Html::from(page.into_string()).into_response()
}
