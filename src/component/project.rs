use crate::prelude::*;

pub async fn get_projects(
    ax::State(state): ax::State<Arc<AppState>>,
    cookie: ax::CookieJar,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let db = &state.db;
    let user = User::from_cookie(&db, &cookie).await;

    println!("GET projects, user = {:?}", user);

    let content = html! {
        (make_posts_table(db, Some("project".to_string()), None, true, false).await)
    };

    let page = make_page(
        Some("Projects"),
        "A list of all projects.",
        vec!["/styles/post.css"],
        content,
        user,
    );

    (
        ax::StatusCode::OK,
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}
