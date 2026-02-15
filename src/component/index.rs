use crate::prelude::*;

pub async fn get_index(
    ax::State(state): ax::State<Arc<AppState>>,
    cookies: ax::CookieJar,
) -> impl IntoResponse {
    let db = &state.db.lock().unwrap();
    let user = User::from_cookie(db, &cookies).ok();

    println!("GET index, user = {:?}", user);

    let posts_table = match make_posts_table(db, None, Some(5), false, true) {
        Ok(posts_table) => posts_table,
        Err(_) => return make_error(500, "Failed to load posts table").into_response(),
    };

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

        (posts_table)
    };

    let page = make_page(
        None,
        "Kai's personal website.",
        vec!["/styles/post.css"],
        content,
        user,
        false,
    );

    ax::Html::from(page.into_string()).into_response()
}
