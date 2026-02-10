use maud::{Markup, PreEscaped, DOCTYPE};

use crate::prelude::*;

pub fn make_page(title: &str, description: &str, content: impl Into<String>) -> Markup {
    html! {
        (DOCTYPE)

        html {
            head {
                title { "Kai - " (title) }
                meta name = "description" content = (description) {}
                meta name = "viewport" content = "width=device-width, initial-scale=1" {}
                link rel = "stylesheet" href = "/static/style.css" {}
                link rel = "icon" href = "/static/favicon.ico" {}
            }

            body {
                (PreEscaped(content.into()))
            }
        }
    }
}
