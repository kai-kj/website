use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub database_path: String,
    pub posts_path: String,
    pub files_path: String,
    pub post_content_path: String,
    pub post_metadata_path: String,
    pub post_assets_path: String,
    pub post_public_photos_path: String,
    pub post_private_photos_path: String,
    pub photo_max_preview_size: u32,
    pub photo_quality: u8,
    pub server_host: String,
    pub server_port: u16,
    pub posts_url: String,
}

impl Config {
    pub fn from_json_str(json_str: &str) -> Config {
        serde_json::from_str(json_str).expect("failed to decode configuration")
    }

    pub fn from_json_file(path: &str) -> Config {
        let json_str = std::fs::read_to_string(path).expect("failed to read configuration file");
        Config::from_json_str(&json_str)
    }
}
