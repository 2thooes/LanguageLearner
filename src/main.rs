use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::env;
use tower_http::services::ServeDir;
use dotenvy::dotenv;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Deserialize)]
struct AuthRequest {
    login: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    message: String,
}

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    jwt_secret: String,
}


async fn init_db(pool: &SqlitePool) -> Result<(),sqlx::Error>{
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            login TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS courses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            description TEXT,
            language TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS levels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            course_id INTEGER,
            level_number INTEGER NOT NULL,
            title TEXT NOT NULL,
            FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS cards (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            level_id INTEGER,
            front TEXT NOT NULL,
            back TEXT NOT NULL,
            audio_url TEXT,
            sort_order INTEGER NOT NULL,
            FOREIGN KEY (level_id) REFERENCES levels(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS user_courses (
            user_id INTEGER,
            course_id INTEGER,
            enrolled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, course_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS user_card_progress (
            user_id INTEGER,
            card_id INTEGER,
            box INTEGER DEFAULT 1,
            status TEXT DEFAULT 'learning',
            due_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, card_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    dotenv().ok();
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}",port);

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:flashcards.db?mode=rwc".to_string());
    let pool = SqlitePool::connect(&database_url).await?;

    init_db(&pool).await?;

    let state = AppState {
        pool,
        jwt_secret,
    };

    let app = Router::new()
        .nest_service("/static", ServeDir::new("frontend/static"))
        // Routes with authorisation checking
        .route("/", get(root_handler))
        .route("/study", get(study_page_handler))
        .route("/register", get(register_page_handler))
        .route("/login", get(login_page_handler))
        // API routes
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
        .with_state(state);
    
    let listener  = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener,app).await.unwrap();
    Ok(())
}


fn create_jwt(user_id: i64, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::days(7)).timestamp() as usize;
    let claims = Claims { sub: user_id.to_string(), exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

fn verify_token(headers: &HeaderMap, secret: &str) -> Option<Claims> {
    let cookies = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    let token = cookies
        .split(';')
        .map(str::trim)
        .find_map(|c| c.strip_prefix("auth_token="))?;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|d| d.claims)
}

async fn root_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_some() {
        Redirect::to("/study").into_response()
    } else {
        Redirect::to("/register").into_response()
    }
}

async fn study_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_none() {
        return Redirect::to("/register").into_response();
    }

    let html = tokio::fs::read_to_string("frontend/study.html")
        .await
        .unwrap();
    Html(html).into_response()
}

async fn register_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_some() {
        return Redirect::to("/study").into_response();
    }

    let html = tokio::fs::read_to_string("frontend/register.html")
        .await
        .unwrap_or_else(|_| "<h1>Page was not found</h1>".to_string());
    Html(html).into_response()
}

async fn login_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_some() {
        return Redirect::to("/study").into_response();
    }

    let html = tokio::fs::read_to_string("frontend/login.html")
        .await
        .unwrap_or_else(|_| "<h1>Page was not found</h1>".to_string());
    Html(html).into_response()
}

// --- API ---

fn internal<E>(_: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, "Внутренняя ошибка".to_string())
}

fn auth_cookie(token: &str) -> String {
    format!("auth_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Lax", token)
}

// Registration

async fn register_handler(State(state): State<AppState>,Json(payload): Json<AuthRequest>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let login = payload.login.trim().to_string();
    let password = payload.password;

    if login.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Empty login".to_string(),
        ));
    }
    if password.len() < 8 || password.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password is too big or too small".to_string(),
        ));
    }

    // Argon2 is heavy, so computing in other thread
    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string())
    })
    .await
    .map_err(internal)?
    .map_err(internal)?;

    let result = sqlx::query("INSERT INTO users (login, password_hash) VALUES (?, ?)")
        .bind(&login)
        .bind(&password_hash)
        .execute(&state.pool)
        .await;

    let user_id = match result {
        Ok(r) => r.last_insert_rowid(),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err((StatusCode::CONFLICT, "Логин уже занят".to_string()));
        }
        Err(e) => return Err(internal(e)),
    };

    let token = create_jwt(user_id, &state.jwt_secret).map_err(internal)?;

    Ok((
        StatusCode::CREATED,
        [(axum::http::header::SET_COOKIE, auth_cookie(&token))],
        Json(AuthResponse { message: "Successfull registration".to_string() }),
    ))
}

async fn login_handler(State(state): State<AppState>,Json(payload): Json<AuthRequest>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let bad_creds = || (StatusCode::UNAUTHORIZED, "Wrong login or password".to_string());//Anonimous func

    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, password_hash FROM users WHERE login = ?")
            .bind(payload.login.trim())
            .fetch_optional(&state.pool)
            .await
            .map_err(internal)?;

    let (user_id, hash) = row.ok_or_else(bad_creds)?;

    if payload.password.len() > 128 {
        return Err(bad_creds());
    }
    let password = payload.password;

    let is_valid = tokio::task::spawn_blocking(move || {
        match argon2::PasswordHash::new(&hash) {
            Ok(parsed) => Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
            Err(_) => false,
        }
    })
    .await
    .map_err(internal)?;

    if !is_valid {
        return Err(bad_creds());
    }

    let token = create_jwt(user_id, &state.jwt_secret).map_err(internal)?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, auth_cookie(&token))],
        Json(AuthResponse { message: "Успешный вход".to_string() }),
    ))
}