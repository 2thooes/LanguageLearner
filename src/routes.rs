use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use crate::structs::AppState;
use crate::handlers;

pub fn get_routes(state: AppState) -> Router{
    return Router::new()
        .nest_service("/static", ServeDir::new("frontend/static"))
        // Routes with authorisation checking
        .route("/", get(handlers::root_handler))
        .route("/study", get(handlers::study_page_handler))
        .route("/register", get(handlers::register_page_handler))
        .route("/login", get(handlers::login_page_handler))
        // API routes
        .route("/api/register", post(handlers::register_handler))
        .route("/api/login", post(handlers::login_handler))
        .route("/api/logout", post(handlers::logout_handler))
        .route("/api/me", get(handlers::me_handler))
        .route("/api/mycourses", get(handlers::my_courses_handler))
        .nest_service("/js", ServeDir::new("frontend/js"))
        .nest_service("/styles", ServeDir::new("frontend/styles"))
        .with_state(state);
}