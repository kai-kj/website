use crate::prelude::*;

pub async fn get_index(
    ax::State(state): ax::State<Arc<AppState>>,
) -> (ax::StatusCode, ax::HeaderMap, ax::Html<String>) {
    let db = &state.db;
    let cfg = &state.config;

    println!("GET index");

    let recent_posts = Post::list(db, Some(5)).await;

    let content = html! {
        h1 { "About me" }

        p {
            "For my master's, I'm currently studying " a href = "https://cbb.ethz.ch/" { "Computational Biology and Bioinformatics" } " at ETH Zurich. I studied " a href = "https://curriculum.maastrichtuniversity.nl/education/bachelor/data-science-and-artificial-intelligence" { "Data Science and AI" } " for my bachelor's at Maastricht University."
        }

        p {
            "I've worked with " a href = "https://www.i-medtech.nl/" { "i-Med Technology"} " for over 2 years, where I've been developing and implementing various image processing techniques for a digital surgical loupe."
        }

        p {
            "I'm half Japanese, half British, and I've lived in the UK, Japan, Spain, the Netherlands, and Switzerland. I can speak English, Spanish, and Japanese."
        }

        h1 { "Recent posts" }

        (make_posts_table(db, None, Some(5), false, true).await)
    };

    let page = make_page(
        None,
        "Kai's personal website.",
        vec!["/styles/post.css"],
        content,
    );

    (
        ax::StatusCode::OK,
        ax::HeaderMap::new(),
        page.into_string().into(),
    )
}
