use crate::prelude::*;

pub fn make_error(
    code: u16,
    message: &str,
    user: Option<User>,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let title = format!("{}", code);
    let message = format!("Error {}: {}", code, message);

    let content = html! {
        section class="error" {
            p { (message)}
            p { a href="/" { "> return home <"} }
        }
    };

    let page = make_page(
        Some(&title),
        &message,
        vec!["/styles/error.css"],
        content,
        user,
    );

    (
        ax::StatusCode::from_u16(code).unwrap(),
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}

pub async fn get_error(
    ax::State(state): ax::State<Arc<AppState>>,
    ax::Query(params): ax::Query<HashMap<String, String>>,
    cookies: ax::CookieJar,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let db = &state.db;

    let code = params
        .get("code")
        .unwrap_or(&"404".to_string())
        .parse::<u16>()
        .unwrap();

    let user = User::from_cookie(db, &cookies).await;
    
    println!("GET error {}, user = {:?}", code, user);

    make_error(code, "Not found", user)
}
