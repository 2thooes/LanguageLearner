use sqlx::SqlitePool;
use sqlx::FromRow;

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

#[derive(Debug, Deserialize)]
pub struct CardCsvRow {
    #[serde(rename = "StudiedLanguage")]
    pub studied_language: String,
    #[serde(rename = "AdditionalInformation")]
    pub additional_info: String,
    #[serde(rename = "StudentsLanguage")]
    pub students_language: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: i64,
    pub login: String,
}

#[derive(Serialize)]
pub struct MyClassesResponse {
    pub courses: Vec<Class>,
}
#[derive(Serialize, FromRow)]
pub struct Class {
    pub id: u64,
    pub title: String,
}

#[derive(Serialize)]
pub struct AvailableClassesResponse{
    pub courses: Vec<Course>,
}

#[derive(Serialize, FromRow)]
pub struct Course{
    pub title: String,
    pub description: String,
    pub user_language_code: String,
    pub target_language_code: String,
}

#[derive(Debug,Deserialize)]
pub struct AddUserCourseRequest {
    pub primary_language: String,
    pub chosen_language: String,
}

#[derive(Deserialize)]
pub struct CourseQuery {
    pub id: i64,
}