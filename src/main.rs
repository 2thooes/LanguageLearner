use sqlx::SqlitePool;
use std::env;
use dotenvy::dotenv;
use structs::{AppState};

mod database_functions;
mod structs;
mod routes;
mod handlers;

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    dotenv().ok();
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}",port);

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:flashcards.db?mode=rwc".to_string());
    let pool = SqlitePool::connect(&database_url).await?;

    database_functions::init_db(&pool).await?;

    let state = AppState {
        pool,
        jwt_secret,
    };

    let app = routes::get_routes(state);
    
    let listener  = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener,app).await.unwrap();
    Ok(())
}

