use crate::prelude::*;

pub fn make_error(code: u16, message: &str) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let title = format!("{}", code);
    let message = format!("Error {}: {}", code, message);

    let content = html! {
        section class="error" {
            p { (message)}
            p { a href="/" { "> return home <"} }
        }
    };

    let page = make_page(Some(&title), &message, vec!["/styles/error.css"], content);

    (
        ax::StatusCode::from_u16(code).unwrap(),
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}

pub async fn get_error(
    ax::Query(params): ax::Query<HashMap<String, String>>,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let code = params
        .get("code")
        .unwrap_or(&"404".to_string())
        .parse::<u16>()
        .unwrap();

    make_error(code, "Not found")
}
