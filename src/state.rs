use crate::prelude::*;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: Arc<Mutex<Config>>,
}
