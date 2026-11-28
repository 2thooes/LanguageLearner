use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
    PasswordVerifier,
};
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect,Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use crate::structs::{AppState, AuthRequest, AuthResponse, Claims,MeResponse,MyClassesResponse,Course,AddUserCourseRequest,Class,CourseQuery};
use crate::database_functions;

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

pub async fn root_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_some() {
        Redirect::to("/study").into_response()
    } else {
        Redirect::to("/register").into_response()
    }
}

pub async fn study_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_none() {
        return Redirect::to("/register").into_response();
    }

    //let options = database_functions::get_all_classes_by_user(&state.pool,"CHANGEME".to_string());

    let html = tokio::fs::read_to_string("frontend/study.html")
        .await
        .unwrap();
    Html(html).into_response()
}

pub async fn register_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if verify_token(&headers, &state.jwt_secret).is_some() {
        return Redirect::to("/study").into_response();
    }

    let html = tokio::fs::read_to_string("frontend/register.html")
        .await
        .unwrap_or_else(|_| "<h1>Page was not found</h1>".to_string());
    Html(html).into_response()
}

pub async fn login_page_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
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
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string())
}

fn auth_cookie(token: &str) -> String {
    format!("auth_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Lax", token)
}

// Registration
pub async fn register_handler(State(state): State<AppState>,Json(payload): Json<AuthRequest>) -> Result<impl IntoResponse, (StatusCode, String)> {
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
            return Err((StatusCode::CONFLICT, "Nickname is taken".to_string()));
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

pub async fn login_handler(State(state): State<AppState>,Json(payload): Json<AuthRequest>) -> Result<impl IntoResponse, (StatusCode, String)> {
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

pub async fn logout_handler() -> impl IntoResponse {
    println!("aaaaaaa");
    let cookie = "auth_token=; HttpOnly; Path=/; Max-Age=0".to_string();

    let response = (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(AuthResponse { message: "Success".to_string() }),
    );

    response.into_response()
}

pub async fn me_handler(State(state): State<AppState>,headers: HeaderMap) -> Result<Json<MeResponse>,Response>{
    let claims = verify_token(&headers, &state.jwt_secret).ok_or(unauthorized_with_clear_cookie())?;
    let user_id: i64 = claims.sub.parse().map_err(|_| unauthorized_with_clear_cookie())?;

    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, login FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| unauthorized_with_clear_cookie())?;

    let (id, login) = match row{
        Some(data) => data,
        None => return Err(unauthorized_with_clear_cookie())?,
    };
    

    Ok(Json(MeResponse { id, login }))
}

pub async fn my_courses_handler(State(state): State<AppState>,headers: HeaderMap) -> Result<Json<MyClassesResponse>,Response>{
    let claims = verify_token(&headers, &state.jwt_secret).ok_or(unauthorized_with_clear_cookie())?;
    let user_id: i64 = claims.sub.parse().map_err(|_| unauthorized_with_clear_cookie())?;

    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, login FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| unauthorized_with_clear_cookie())?;

    let (id, login) = match row{
        Some(data) => data,
        None => return Err(unauthorized_with_clear_cookie()),
    };
    // 2. Одним запросом через JOIN получаем все названия курсов пользователя
    let mut courses: Vec<Class> = sqlx::query_as("SELECT c.title, id FROM courses c JOIN user_courses uc ON c.id = uc.course_id WHERE uc.user_id = ?")
        .bind(user_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| unauthorized_with_clear_cookie())?;

    Ok(Json(MyClassesResponse {courses}))
}

pub async fn available_courses_hendler(State(state): State<AppState>,headers: HeaderMap) -> Result<Json<Vec<Course>>,Response>{
    let claims = verify_token(&headers, &state.jwt_secret)
        .ok_or_else(unauthorized_with_clear_cookie)?;

    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| unauthorized_with_clear_cookie())?;

    // Дополнительно проверяем, что пользователь всё ещё существует в БД
    let user_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "DB Error".to_string()).into_response()
        })?;

    if user_exists.is_none() {
        return Err(unauthorized_with_clear_cookie());
    }

    let courses: Vec<Course> = sqlx::query_as(
        "SELECT
            c.title,
            c.description,
            c.user_language_code,
            c.target_language_code
         FROM courses c
         WHERE NOT EXISTS (
             SELECT 1
             FROM user_courses uc
             WHERE uc.course_id = c.id
               AND uc.user_id = ?
         )",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "DB Error".to_string()).into_response()
    })?;
    Ok(Json(courses))
}

pub async fn add_user_courses(State(state): State<AppState>,headers: HeaderMap,Json(payload): Json<AddUserCourseRequest>) -> Result<Json<String>,Response>{
    println!("получен запрос на добавление курса");
    let claims = verify_token(&headers, &state.jwt_secret).ok_or(unauthorized_with_clear_cookie())?;
    let user_id: i64 = claims.sub.parse().map_err(|_| unauthorized_with_clear_cookie())?;
    let res = database_functions::add_new_course_for_user(&state.pool,user_id,&payload.primary_language,&payload.chosen_language)
    .await
    .map_err(|_|{
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
        .into_response()
    })?;

    if !res {
        return Err((StatusCode::NOT_FOUND, "Course not found".to_string()).into_response());
    }

    Ok(Json("Ok".to_string()))
}

pub async fn get_course_content(State(state): State<AppState>,Query(params): Query<CourseQuery>,headers: HeaderMap) -> Result<Json<Vec<String>>,Response>{
    let claims = verify_token(&headers, &state.jwt_secret).ok_or(unauthorized_with_clear_cookie())?;
    let user_id: i64 = claims.sub.parse().map_err(|_| unauthorized_with_clear_cookie())?;
    //let response = sqlx::query_as("SELECT title FROM levels WHERE ");
    let course_id: i64 = params.id;
    println!("{}",course_id);
    Ok(vec!["qwe".to_string()].into())
}

fn unauthorized_with_clear_cookie() -> Response {
    let cookie = "auth_token=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax".to_string();
    (
        StatusCode::UNAUTHORIZED,
        [(header::SET_COOKIE, cookie)],
        "Unauthorized".to_string(),
    ).into_response()
}