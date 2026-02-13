use crate::prelude::*;

pub fn make_error(code: u16, message: &str, user: Option<User>) -> impl IntoResponse {
    let title = format!("{}", code);
    let message = format!("Error {}: {}", code, message);
    let code = ax::StatusCode::from_u16(code).unwrap_or(ax::StatusCode::INTERNAL_SERVER_ERROR);

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

    (code, page.into_string()).into_response()
}

pub async fn get_not_found(
    ax::State(state): ax::State<Arc<AppState>>,
    request: ax::Request,
) -> impl IntoResponse {
    let db = &state.db;

    let cookies = request
        .extensions()
        .get::<ax::CookieJar>()
        .cloned()
        .unwrap_or(ax::CookieJar::new());

    let params = request
        .extensions()
        .get()
        .cloned()
        .unwrap_or(ax::Query::<HashMap<String, String>>::default());

    let user = User::from_cookie(db, &cookies).await;
    let path = request.uri().path();
    let code = params
        .get("code")
        .unwrap_or(&"404".to_string())
        .parse::<u16>()
        .unwrap();

    println!("GET error {}, user = {:?}", code, user);

    if !path.ends_with('/') && code == 404 {
        println!("  redirecting with trailing slash");
        return ax::Redirect::to(&format!("{}/", path)).into_response();
    }

    make_error(code, "Not found", user).into_response()
}
