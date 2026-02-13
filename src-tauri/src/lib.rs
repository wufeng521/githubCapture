mod trending;
mod db;
mod ai;
mod search;
mod models;
mod llm;
mod config;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use config::{ConfigManager, commands::ConfigManagerState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default()
            .add_migrations("sqlite:github_capture.db", db::get_migrations())
            .build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // 初始化配置管理器
            let handle = app.handle().clone();
            let manager = tauri::async_runtime::block_on(async move {
                ConfigManager::new(handle).await
            })?;
            
            let manager_state: ConfigManagerState = Arc::new(Mutex::new(manager));
            app.manage(manager_state);

            // 初始化数据库连接池
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");
            let db_path = app_data_dir.join("github_capture.db");
            
            // 打印路径用于调试确认（在某些环境下很有用）
            println!("Database path: {:?}", db_path);
            
            use sqlx::sqlite::SqliteConnectOptions;
            let pool = tauri::async_runtime::block_on(async move {
                let options = SqliteConnectOptions::new()
                    .filename(&db_path)
                    .create_if_missing(true);
                
                sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(options)
                    .await
            }).expect("Failed to connect to database");
            
            app.manage(pool.clone());

            // 确保执行迁移
            let pool_clone = pool.clone();
            tauri::async_runtime::block_on(async move {
                db::run_migrations(&pool_clone).await
            }).expect("Failed to run migrations");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            trending::get_trending,
            ai::summarize_repo,
            ai::test_model_connection,
            ai::list_models,
            ai::get_cached_insight,
            ai::check_insights_batch,
            search::ai_rewrite_query,
            search::search_github,
            // 配置管理命令
            config::commands::get_model_configs,
            config::commands::get_active_model_config,
            config::commands::set_active_model_config,
            config::commands::save_model_config,
            config::commands::update_model_config,
            config::commands::delete_model_config,
            config::commands::clear_model_cache,
            // 数据库收藏命令
            db::toggle_favorite,
            db::get_favorites,
            db::is_favorite,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}