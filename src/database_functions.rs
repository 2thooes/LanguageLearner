use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use crate::structs::CardCsvRow;

async fn get_or_create_level(pool: &SqlitePool,course_id: i64,level_number: i32,title: &str) -> Result<i64, sqlx::Error> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM levels WHERE course_id = ? AND title = ? LIMIT 1")
            .bind(course_id)
            .bind(title)
            .fetch_optional(pool)
            .await?;

    if let Some((id,)) = row {
        println!("[DB][levels] уже есть: id={}, '{}'", id, title);
        return Ok(id);
    }

    let res = sqlx::query("INSERT INTO levels (course_id, level_number, title) VALUES (?, ?, ?)")
        .bind(course_id)
        .bind(level_number)
        .bind(title)
        .execute(pool)
        .await?;
    let id = res.last_insert_rowid();
    println!("[DB][levels] INSERT: id={}, number={}, '{}'", id, level_number, title);
    Ok(id)
}
/// returns true, if card was added, false, if it was already existing
async fn insert_card_if_new(pool: &SqlitePool,level_id: i64,front: &str,back: &str,sort_order: i32) -> Result<bool, sqlx::Error> {
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM cards WHERE level_id = ? AND front = ? AND back = ? LIMIT 1",
    )
    .bind(level_id)
    .bind(front)
    .bind(back)
    .fetch_optional(pool)
    .await?;

    if exists.is_some() {
        return Ok(false);
    }

    let res = sqlx::query(
        "INSERT INTO cards (level_id, front, back, sort_order) VALUES (?, ?, ?, ?)",
    )
    .bind(level_id)
    .bind(front)
    .bind(back)
    .bind(sort_order)
    .execute(pool)
    .await?;

    println!(
        "[DB][cards] INSERT id={} level={} order={}: '{}' | '{}'",
        res.last_insert_rowid(), level_id, sort_order, front, back
    );
    Ok(true)
}

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
            user_language_code TEXT NOT NULL,
            target_language_code TEXT NOT NULL
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
    init_cards(pool).await;
    Ok(())
}

async fn init_cards(pool: &SqlitePool){
    println!("itin_cards");
    import_course_from_dir(pool,"cards_data/ru-ch","ru","ch","Китайский HSK 3.0","Курс по китайскому для русскоговорящих").await;
    import_course_from_dir(pool,"cards_data/en-ch","en","ch","Chinese HSK 3.0","Chinese course for english speakers").await;
    
}

async fn import_course_from_dir(pool: &SqlitePool,dir_path: &str,user_language_code: &str,target_language_code: &str,course_title: &str,course_description: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(dir_path);
    if !path.is_dir() {
        println!("[IMPORT] Папка '{}' не найдена, пропускаем импорт.", dir_path);
        return Ok(());
    }

    println!("[IMPORT] ===== Старт импорта из '{}' =====", dir_path);

    // 1. Курс
    let course_row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM courses WHERE title = ? LIMIT 1")
            .bind(course_title)
            .fetch_optional(pool)
            .await?;

    let course_id = match course_row {
        Some(row) => {
            println!(
                "[DB][courses] уже есть: id={}, title='{}'",
                row.0, course_title
            );
            row.0
        }
        None => {
            let res = sqlx::query(
                "INSERT INTO courses (title, description, user_language_code, target_language_code) VALUES (?, ?, ?, ?)",
            )
            .bind(course_title)
            .bind(course_description)
            .bind(user_language_code)
            .bind(target_language_code)
            .execute(pool)
            .await?;
            let id = res.last_insert_rowid();
            println!(
                "[DB][courses] INSERT: id={}, title='{}', description='{}', user_language_code='{}', target_language_code='{}'",
                id, course_title, course_description, user_language_code,target_language_code
            );
            id
        }
    };

    // 2. Файлы
    let mut csv_files: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("csv"))
        .collect();

    csv_files.sort();
    println!("[IMPORT] Найдено CSV-файлов: {}", csv_files.len());

    let mut total_added = 0;
    let mut total_skipped = 0;

    // 3. Уровни
        // 3. Каждый файл -> два уровня: Speaking и Reading
    for (file_idx, file_path) in csv_files.into_iter().enumerate() {
        let base = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Level")
            .to_string();

        println!("[IMPORT] --- Файл '{}' ---", file_path.display());

        let speaking_id = get_or_create_level(
            pool, course_id, (file_idx * 2 + 1) as i32, &format!("Speaking {}", base),
        ).await?;
        let reading_id = get_or_create_level(
            pool, course_id, (file_idx * 2 + 2) as i32, &format!("Reading {}", base),
        ).await?;

        let mut rdr = csv::Reader::from_path(&file_path)?;
        let mut added = 0;
        let mut skipped = 0;

        for (card_idx, result) in rdr.deserialize().enumerate() {
            let row: CardCsvRow = result?;

            let known = row.students_language.trim();   // английский
            let studied = row.studied_language.trim();  // китайский
            let info = row.additional_info.trim();      // пиньинь
            let sort_order = (card_idx + 1) as i32;

            // Speaking: английский -> китайский (+ пиньинь)
            let speaking_back = if info.is_empty() {
                studied.to_string()
            } else {
                format!("{} ({})", studied, info)
            };

            // Reading: китайский -> пиньинь + английский
            let reading_back = if info.is_empty() {
                known.to_string()
            } else {
                format!("{} — {}", info, known)
            };

            for (level_id, front, back) in [
                (speaking_id, known, speaking_back.as_str()),
                (reading_id, studied, reading_back.as_str()),
            ] {
                if insert_card_if_new(pool, level_id, front, back, sort_order).await? {
                    added += 1;
                } else {
                    skipped += 1;
                }
            }
        }

        println!(
            "[IMPORT] '{}': добавлено {}, пропущено {}",
            base, added, skipped
        );
        total_added += added;
        total_skipped += skipped;
    }

    Ok(())
}

pub async fn add_new_course_for_user(pool: &SqlitePool,user_id: i64,primary_language: &str,chosen_language: &str) -> Result<bool,sqlx::Error>{
    let course_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM courses WHERE user_language_code = ? AND target_language_code = ?",
    )
    .bind(primary_language)
    .bind(chosen_language)
    .fetch_optional(pool)
    .await?;

    let Some(course_id) = course_id else {
        return Ok(false);
    };

    sqlx::query("INSERT OR IGNORE INTO user_courses (user_id, course_id) VALUES (?, ?)")
        .bind(user_id)
        .bind(course_id)
        .execute(pool)
        .await?;

    Ok(true)
}


