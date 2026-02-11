use maud::{Markup, PreEscaped, DOCTYPE};

use crate::prelude::*;

pub fn make_page(
    title: Option<&str>,
    description: &str,
    additional_styles: Vec<&str>,
    content: impl Into<String>,
) -> Markup {
    html! {
        (DOCTYPE)

        html {
            head {
                @if let Some(title) = title {
                    title { "Kai - " (title) }
                } @else {
                    title { "Kai" }
                }
                meta name="description" content=(description) {}
                meta name="viewport" content="width=device-width, initial-scale=1" {}
                link rel="icon" href="/assets/logo.jpg" {}
                link rel="stylesheet" href="/styles/page.css" {}
                 @for additional_style in additional_styles {
                    link rel="stylesheet" href=(additional_style) {}
                }
            }

            body {
                nav {
                    a href="/" id="nav-left" {
                        img src="/assets/logo.jpg" alt = "logo" {}
                        div {
                            div { "Kai" }
                            div { "Kitagawa-Jones"}
                        }
                    }
                    div id="nav-right" {
                        a href="/posts/" { "Posts" }
                        a href="/projects/" { "Projects" }
                        a href="/photos/" { "Photos" }
                    }
                }

                @if let Some(title) = title {
                    header { h1 { (title) } }
                }

                main {
                    (PreEscaped(content.into()))
                }

                footer {
                    div {
                        img class="icon" src="/assets/github.svg" alt="github" {}
                        a href="https://github.com/kai-kj" { "kai-kj" }
                    }
                    div {
                        img class="icon" src="/assets/linkedin.svg" alt="linkedin" {}
                        a href="https://linkedin.com/in/kaikitagawajones/" { "Kai Kitagawa-Jones" }
                    }
                    div {
                        img class="icon" src="/assets/mail.svg" alt="mail" {}
                        a href="mailto:kaikitagawajones@gmail.com" { "kaikitagawajones@gmail.com" }
                    }
                }
            }
        }
    }
}
