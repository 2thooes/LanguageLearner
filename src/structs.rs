use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub jwt_secret: String,
}