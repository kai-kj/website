pub mod asset;
pub mod error;
pub mod file;
pub mod index;
pub mod page;
pub mod photo;
pub mod post;
pub mod project;
pub mod user;

pub mod prelude {
    pub use super::asset::{get_asset, Asset};
    pub use super::error::{get_error, make_error};
    pub use super::file::{
        get_asset as get_file_asset, get_file as get_file_file, get_style as get_file_style, File,
    };
    pub use super::index::get_index;
    pub use super::page::make_page;
    pub use super::photo::{get_photo, get_photos, Photo};
    pub use super::post::{get_post, get_posts, make_posts_table, Post};
    pub use super::project::get_projects;
    pub use super::user::{get_login, post_login, post_logout, User};
}
