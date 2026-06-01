use std::sync::Arc;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::state::{SessionInfo, WebState};
use dbx_core::connection::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct AuthCheckResponse {
    pub authenticated: bool,
    pub required: bool,
    pub setup_required: bool,
    pub google_auth_enabled: bool,
    pub user_email: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

const MAX_ATTEMPTS: u32 = 5;
const LOCKOUT_SECS: u64 = 60;

pub async fn login(State(state): State<Arc<WebState>>, Json(body): Json<LoginRequest>) -> Result<Response, StatusCode> {
    let hash_guard = state.password_hash.read().await;
    let hash_str = match hash_guard.as_deref() {
        Some(h) => h.to_string(),
        None => {
            return Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response());
        }
    };
    drop(hash_guard);

    // Check rate limit
    {
        let rl = state.login_rate_limit.lock().await;
        if let Some(locked_until) = rl.locked_until {
            if locked_until > std::time::Instant::now() {
                let remaining = (locked_until - std::time::Instant::now()).as_secs();
                return Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({"error": format!("请 {remaining} 秒后再试")})),
                )
                    .into_response());
            }
        }
    }

    let parsed_hash = PasswordHash::new(&hash_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).is_err() {
        let mut rl = state.login_rate_limit.lock().await;
        rl.fail_count += 1;
        if rl.fail_count >= MAX_ATTEMPTS {
            rl.locked_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(LOCKOUT_SECS));
            rl.fail_count = 0;
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Success — reset rate limit
    {
        let mut rl = state.login_rate_limit.lock().await;
        rl.fail_count = 0;
        rl.locked_until = None;
    }

    let token = uuid::Uuid::new_v4().to_string();
    let session_info = SessionInfo {
        token: token.clone(),
        user_id: "default".to_string(),
        email: "admin@dbx.local".to_string(),
    };
    state.sessions.write().await.insert(token.clone(), session_info);

    let cookie = format!("dbx_session={token}; Path=/; HttpOnly; SameSite=Lax");
    Ok((StatusCode::OK, [("set-cookie", cookie.as_str())], Json(serde_json::json!({"ok": true}))).into_response())
}

pub async fn setup(State(state): State<Arc<WebState>>, Json(body): Json<LoginRequest>) -> Result<Response, StatusCode> {
    // Only allow setup when no password is configured
    if state.password_hash.read().await.is_some() {
        return Err(StatusCode::FORBIDDEN);
    }

    if body.password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // Save to database
    state.default_app.storage.save_password_hash(&hash).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update in-memory state
    *state.password_hash.write().await = Some(hash);

    // Auto-login: create session
    let token = uuid::Uuid::new_v4().to_string();
    let session_info = SessionInfo {
        token: token.clone(),
        user_id: "default".to_string(),
        email: "admin@dbx.local".to_string(),
    };
    state.sessions.write().await.insert(token.clone(), session_info);

    let cookie = format!("dbx_session={token}; Path=/; HttpOnly; SameSite=Lax");
    Ok((StatusCode::OK, [("set-cookie", cookie.as_str())], Json(serde_json::json!({"ok": true}))).into_response())
}

pub async fn check(State(state): State<Arc<WebState>>, req: Request<axum::body::Body>) -> Json<AuthCheckResponse> {
    let google_enabled = state.oauth_config.is_some();
    if google_enabled {
        let token = extract_session_token(&req);
        let session = match token {
            Some(t) => {
                // Read from sessions map inside a block to release read lock quickly
                let sessions_guard = futures::executor::block_on(async {
                    state.sessions.read().await
                });
                sessions_guard.get(&t).cloned()
            }
            None => None,
        };
        let authenticated = session.is_some();
        let user_email = session.map(|s| s.email);
        return Json(AuthCheckResponse {
            authenticated,
            required: true,
            setup_required: false,
            google_auth_enabled: true,
            user_email,
        });
    }

    let has_password = state.password_hash.read().await.is_some();
    if !has_password {
        return Json(AuthCheckResponse {
            authenticated: false,
            required: false,
            setup_required: true,
            google_auth_enabled: false,
            user_email: None,
        });
    }
    let authenticated = match extract_session_token(&req) {
        Some(token) => {
            let sessions_guard = futures::executor::block_on(async {
                state.sessions.read().await
            });
            sessions_guard.contains_key(&token)
        }
        None => false,
    };
    Json(AuthCheckResponse {
        authenticated,
        required: true,
        setup_required: false,
        google_auth_enabled: false,
        user_email: None,
    })
}

pub async fn change_password(
    State(state): State<Arc<WebState>>,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Response, StatusCode> {
    let hash_guard = state.password_hash.read().await;
    let hash_str = match hash_guard.as_deref() {
        Some(h) => h.to_string(),
        None => return Err(StatusCode::BAD_REQUEST),
    };
    drop(hash_guard);

    if body.new_password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let parsed_hash = PasswordHash::new(&hash_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if Argon2::default().verify_password(body.old_password.as_bytes(), &parsed_hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(body.new_password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    state.default_app.storage.save_password_hash(&new_hash).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *state.password_hash.write().await = Some(new_hash);

    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response())
}

pub async fn logout(State(state): State<Arc<WebState>>, req: Request<axum::body::Body>) -> Response {
    if let Some(token) = extract_session_token(&req) {
        state.sessions.write().await.remove(&token);
    }
    let cookie = "dbx_session=; Path=/; HttpOnly; Max-Age=0";
    (StatusCode::OK, [("set-cookie", cookie)], Json(serde_json::json!({"ok": true}))).into_response()
}

pub async fn google_login(State(state): State<Arc<WebState>>) -> Result<Response, StatusCode> {
    let oauth = match &state.oauth_config {
        Some(o) => o,
        None => return Err(StatusCode::NOT_FOUND),
    };
    let csrf_state = uuid::Uuid::new_v4().to_string();
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&\
         redirect_uri={}&\
         response_type=code&\
         scope=openid%20email%20profile&\
         state={}",
        oauth.client_id,
        oauth.redirect_uri,
        csrf_state
    );
    let cookie = format!("dbx_oauth_state={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=300", csrf_state);
    Ok((
        StatusCode::FOUND,
        [("location", auth_url.as_str()), ("set-cookie", cookie.as_str())],
    ).into_response())
}

pub async fn google_callback(
    State(state): State<Arc<WebState>>,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
    req: Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    let cookie_state = extract_cookie(&req, "dbx_oauth_state");
    if cookie_state.is_none() || cookie_state.unwrap() != params.state {
        return Err(StatusCode::BAD_REQUEST);
    }

    let oauth = match &state.oauth_config {
        Some(o) => o,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let client = reqwest::Client::new();
    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", &oauth.client_id),
            ("client_secret", &oauth.client_secret),
            ("code", &params.code),
            ("grant_type", &"authorization_code".to_string()),
            ("redirect_uri", &oauth.redirect_uri),
        ])
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !token_res.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
    }
    let token_data: TokenResponse = token_res.json().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let userinfo_res = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token_data.access_token)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !userinfo_res.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    #[derive(Deserialize)]
    struct UserInfo {
        email: String,
    }
    let user_info: UserInfo = userinfo_res.json().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = uuid::Uuid::new_v4().to_string();
    let sanitized_email = user_info.email.replace(|c: char| !c.is_alphanumeric() && c != '@' && c != '.', "_");

    let session_info = SessionInfo {
        token: token.clone(),
        user_id: sanitized_email,
        email: user_info.email,
    };

    state.sessions.write().await.insert(token.clone(), session_info);

    let cookie = format!("dbx_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000");
    let state_cookie = "dbx_oauth_state=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        StatusCode::FOUND,
        [
            ("location", "/"),
            ("set-cookie", cookie.as_str()),
            ("set-cookie", state_cookie),
        ],
    ).into_response())
}

fn extract_session_token<B>(req: &Request<B>) -> Option<String> {
    extract_cookie(req, "dbx_session")
}

fn extract_cookie<B>(req: &Request<B>, name: &str) -> Option<String> {
    let cookie_header = req.headers().get("cookie")?.to_str().ok()?;
    let prefix = format!("{name}=");
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(&prefix) {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub async fn auth_middleware(
    State(state): State<Arc<WebState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Auth endpoints are always accessible
    let path = req.uri().path();
    if path.starts_with("/api/auth/") {
        return next.run(req).await;
    }

    // Non-API requests (static files) are always accessible
    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    // If Google Auth is enabled
    let google_enabled = state.oauth_config.is_some();

    // Check session token
    let session_info = if let Some(token) = extract_session_token(&req) {
        state.sessions.read().await.get(&token).cloned()
    } else {
        None
    };

    if google_enabled {
        if let Some(session) = session_info {
            // 1. Get or create isolated AppState for this user
            let user_app = {
                let apps = state.user_apps.read().await;
                if let Some(app) = apps.get(&session.user_id) {
                    app.clone()
                } else {
                    drop(apps);
                    let mut apps = state.user_apps.write().await;
                    if let Some(app) = apps.get(&session.user_id) {
                        app.clone()
                    } else {
                        let user_dir = state.data_dir.join("users").join(&session.user_id);
                        std::fs::create_dir_all(&user_dir).expect("Failed to create user directory");
                        
                        let db_path = user_dir.join("dbx.db");
                        let storage = dbx_core::storage::Storage::open(&db_path).await.expect("Failed to open user storage");
                        storage.migrate_from_json(&user_dir).await.expect("Failed to migrate JSON data");
                        
                        let app = Arc::new(AppState::new_with_plugin_and_agent_dir_and_app_version(
                            storage,
                            user_dir.join("plugins"),
                            user_dir.join("agents"),
                            env!("CARGO_PKG_VERSION"),
                        ));
                        apps.insert(session.user_id.clone(), app.clone());
                        app
                    }
                }
            };

            // 2. Bind the user's isolated AppState to task-local CURRENT_APP_STATE
            return crate::state::CURRENT_APP_STATE.scope(user_app, next.run(req)).await;
        }
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Local password-based auth
    // No password set — allow everything (using default_app)
    if state.password_hash.read().await.is_none() {
        return next.run(req).await;
    }

    if session_info.is_some() {
        return next.run(req).await;
    }

    StatusCode::UNAUTHORIZED.into_response()
}
