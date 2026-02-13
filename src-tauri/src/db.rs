use sqlx::sqlite::SqlitePool;
use tauri_plugin_sql::{Migration, MigrationKind};

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create initial tables",
            sql: "
                CREATE TABLE IF NOT EXISTS repos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    author TEXT NOT NULL,
                    name TEXT NOT NULL,
                    description TEXT,
                    language TEXT,
                    stars TEXT,
                    forks TEXT,
                    url TEXT UNIQUE,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS insights (
                    repo_url TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(repo_url) REFERENCES repos(url)
                );
                CREATE TABLE IF NOT EXISTS search_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    query TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
            ",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "add stars and forks to repos table",
            sql: "
                ALTER TABLE repos ADD COLUMN stars TEXT;
                ALTER TABLE repos ADD COLUMN forks TEXT;
            ",
            kind: MigrationKind::Up,
        }
    ]
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), String> {
    // 简单的迁移逻辑：按顺序执行所有 SQL
    // 注意：这里没有像 tauri-plugin-sql 那样追踪版本，
    // 主要是为了确保字段一定存在。在生产环境应当使用专业的迁移追踪。
    let migrations = get_migrations();
    for m in migrations {
        // 分割多条 SQL 语句（简单的按分号分割）
        for sql in m.sql.split(';') {
            let sql = sql.trim();
            if sql.is_empty() { continue; }
            
            // 执行 SQL，忽略已存在的错误（例如字段已存在）
            let _ = sqlx::query(sql).execute(pool).await;
        }
    }
    Ok(())
}

pub type DbState = SqlitePool;

#[tauri::command]
pub async fn toggle_favorite(
    repo: crate::ai::RepoInfo,
    db: tauri::State<'_, DbState>,
) -> Result<bool, String> {
    // 检查是否存在
    let existing = sqlx::query("SELECT id FROM repos WHERE url = ?")
        .bind(&repo.url)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    if existing.is_some() {
        // 删除
        sqlx::query("DELETE FROM repos WHERE url = ?")
            .bind(&repo.url)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        // 插入
        sqlx::query("INSERT INTO repos (author, name, description, language, url, stars, forks) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&repo.author)
            .bind(&repo.name)
            .bind(&repo.description)
            .bind(&repo.language)
            .bind(&repo.url)
            .bind(&repo.stars)
            .bind(&repo.forks)
            .execute(db.inner())
            .await
            .map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn get_favorites(
    db: tauri::State<'_, DbState>,
) -> Result<Vec<crate::trending::TrendingRepo>, String> {
    let rows = sqlx::query_as::<_, crate::trending::TrendingRepo>("SELECT author, name, description, language, COALESCE(stars, '') as stars, COALESCE(forks, '') as forks, '' as stars_today, url, 'Favorite' as topic FROM repos ORDER BY created_at DESC")
        .fetch_all(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

#[tauri::command]
pub async fn is_favorite(
    url: String,
    db: tauri::State<'_, DbState>,
) -> Result<bool, String> {
    let existing = sqlx::query("SELECT id FROM repos WHERE url = ?")
        .bind(&url)
        .fetch_optional(db.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(existing.is_some())
}
