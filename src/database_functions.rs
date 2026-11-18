use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> Result<(),sqlx::Error>{
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
    init_cards();
    Ok(())
}

async fn init_cards(){
    
}