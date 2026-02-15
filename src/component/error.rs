use crate::prelude::*;

pub fn make_error(code: u16, message: &str) -> impl IntoResponse {
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
        None,
        true,
    );

    (code, ax::Html::from(page.into_string())).into_response()
}

pub async fn get_not_found(
    uri: ax::Uri,
    ax::Query(params): ax::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let uri = uri.path();
    let code = params
        .get("code")
        .unwrap_or(&"404".to_string())
        .parse::<u16>()
        .unwrap();

    println!("GET error {}", code);

    if !uri.ends_with('/') && code == 404 {
        println!("redirecting with trailing slash");
        return ax::Redirect::to(&format!("{}/", uri)).into_response();
    }

    make_error(code, "Page not found").into_response()
}
