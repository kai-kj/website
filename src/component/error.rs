use crate::prelude::*;

pub fn make_error(code: u16, message: &str) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let content = html! {
        h1 { (code) }
        p { (message) }
    };

    let page = make_page("Error", message, content);

    (
        ax::StatusCode::from_u16(code).unwrap(),
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}
