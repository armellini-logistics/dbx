use dbx_core::connection::AppState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_util::sync::CancellationToken;

tokio::task_local! {
    pub static CURRENT_APP_STATE: Arc<AppState>;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub token: String,
    pub user_id: String,
    pub email: String,
}

#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

pub struct LoginRateLimit {
    pub fail_count: u32,
    pub locked_until: Option<std::time::Instant>,
}

pub struct WebState {
    pub default_app: Arc<AppState>,
    pub user_apps: RwLock<HashMap<String, Arc<AppState>>>,
    pub data_dir: PathBuf,
    pub password_hash: RwLock<Option<String>>,
    pub sessions: RwLock<HashMap<String, SessionInfo>>,
    pub sse_channels: RwLock<HashMap<String, broadcast::Sender<String>>>,
    pub sql_file_executions: RwLock<HashMap<String, CancellationToken>>,
    pub login_rate_limit: Mutex<LoginRateLimit>,
    pub oauth_config: Option<OAuthConfig>,
}

impl WebState {
    pub fn app(&self) -> Arc<AppState> {
        if let Ok(app) = CURRENT_APP_STATE.try_with(|app| app.clone()) {
            app
        } else {
            self.default_app.clone()
        }
    }

    pub async fn remove_sse_channel(&self, id: &str) {
        self.sse_channels.write().await.remove(id);
    }
}
