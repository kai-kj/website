pub mod asset;
pub mod file;
pub mod index;
pub mod page;
pub mod photo;
pub mod post;
pub mod error;

pub mod prelude {
    pub use super::asset::{Asset, get_asset};
    pub use super::file::File;
    pub use super::index::get_index;
    pub use super::page::make_page;
    pub use super::photo::{Photo, get_photo};
    pub use super::post::{Post, get_post};
}
