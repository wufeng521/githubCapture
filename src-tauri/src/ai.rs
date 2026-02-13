use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use crate::models::ChatMessage;
use crate::llm::{LLMFactory, LLMResponse, StreamChunk};
use crate::config::commands::ConfigManagerState;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoInfo {
    pub author: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub url: String,
    pub stars: Option<String>,
    pub forks: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamPayload {
    Token(String),
    Error(String),
    Done,
}

/// 向后兼容的仓库总结命令
/// 
/// 增加了 deep_context 和 force_refresh 参数支持
#[tauri::command]
pub async fn summarize_repo(
    repo: RepoInfo,
    api_key: Option<String>,
    model_config_id: Option<String>,
    deep_context: Option<bool>,
    force_refresh: Option<bool>,
    on_event: Channel<StreamPayload>,
    config_manager: tauri::State<'_, ConfigManagerState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let deep_mode = deep_context.unwrap_or(false);
    let refresh = force_refresh.unwrap_or(false);

    // 1. 检查缓存（如果不强制刷新）
    if !refresh {
        if let Some(cached) = get_cached_insight_internal(&repo, &app_handle).await {
            let _ = on_event.send(StreamPayload::Token(cached));
            let _ = on_event.send(StreamPayload::Done);
            return Ok(());
        }
    }

    // 2. 获取基础上下文：README
    // 如果启用深度模式，不再限制 README 长度
    let readme_limit = if deep_mode { None } else { Some(2000) };
    let readme_content = fetch_readme_with_limit(&repo.author, &repo.name, readme_limit).await.unwrap_or_default();
    
    // 3. 获取深度上下文：文件树和核心配置（如果启用）
    let mut extra_context = String::new();
    if deep_mode {
        if let Some(tree) = fetch_tree(&repo.author, &repo.name).await {
            extra_context.push_str("\n\n项目目录结构（部分）：\n---\n");
            extra_context.push_str(&tree);
            extra_context.push_str("\n---");
        }
        
        // 尝试抓取技术栈配置文件
        let config_files = ["package.json", "Cargo.toml", "go.mod", "requirements.txt", "pom.xml"];
        for file in config_files {
            if let Some(content) = fetch_file_content(&repo.author, &repo.name, file, Some(1500)).await {
                extra_context.push_str(&format!("\n\n配置文件 {} 内容片段：\n---\n{}\n---", file, content));
                break; // 拿到一个核心配置即可
            }
        }
    }

    let readme_prompt = if !readme_content.is_empty() {
        format!("\n\n项目 README 内容{}：\n---\n{}\n---", 
            if deep_mode { "（完整）" } else { "（片段）" },
            readme_content
        )
    } else {
        "".to_string()
    };

    let prompt = format!(
        "请对以下 GitHub 项目进行深入浅出的深度总结：\n项目：{}/{}\n描述：{}\n语言：{}{}{}\n\n请包含以下维度：\n1. 核心技术架构\n2. 解决了什么核心痛点\n3. 适合谁用以及如何快速上手（3句话以内）\n请使用 Markdown 格式。",
        repo.author, repo.name, repo.description, repo.language, readme_prompt, extra_context
    );

    let messages = vec![
        ChatMessage::system("你是一个资深的软件架构师和技术布道者，擅长简明扼要地总结技术项目。"),
        ChatMessage::user(&prompt),
    ];

    // 确定使用哪种模式
    let result = if let Some(config_id) = model_config_id {
        summarize_and_cache(config_id, messages, on_event, &config_manager, &repo, &app_handle).await
    } else if let Some(api_key) = api_key {
        // 旧模式暂不支持缓存，保持原有逻辑
        summarize_with_api_key(api_key, messages, on_event).await
    } else {
        return Err("必须提供 API Key 或模型配置 ID".to_string());
    };

    result.map_err(|e| e.to_string())
}

/// 专门用于带缓存的总结逻辑
async fn summarize_and_cache(
    config_id: String,
    messages: Vec<ChatMessage>,
    on_event: Channel<StreamPayload>,
    config_manager: &tauri::State<'_, ConfigManagerState>,
    repo: &RepoInfo,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let manager_lock = config_manager.lock().await;
    let configs = manager_lock.get_all_model_configs().await.map_err(|e| e.to_string())?;
    let config = configs.iter().find(|c| c.id == config_id).ok_or_else(|| format!("找不到模型配置: {}", config_id))?;
    let provider = LLMFactory::create_provider(config).map_err(|e| e.to_string())?;

    let response = provider.chat_completion(messages, &config.default_model, true)
        .await
        .map_err(|e| e.to_string())?;

    let mut full_insight = String::new();

    match response {
        LLMResponse::Completion { content, .. } => {
            let _ = on_event.send(StreamPayload::Token(content.clone()));
            let _ = on_event.send(StreamPayload::Done);
            save_cache(repo, &content, app_handle).await;
            Ok(())
        }
        LLMResponse::Stream { mut stream } => {
            while let Some(chunk) = stream.recv().await {
                match chunk {
                    StreamChunk::Text(text) => {
                        full_insight.push_str(&text);
                        let _ = on_event.send(StreamPayload::Token(text));
                    }
                    StreamChunk::Error(err) => {
                        let _ = on_event.send(StreamPayload::Error(err));
                        return Err("流式响应错误".to_string());
                    }
                    StreamChunk::Done => {
                        let _ = on_event.send(StreamPayload::Done);
                        save_cache(repo, &full_insight, app_handle).await;
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}

/// 暴露给前端的获取缓存命令
#[tauri::command]
pub async fn get_cached_insight(
    repo: RepoInfo,
    app_handle: tauri::AppHandle,
) -> Result<Option<String>, String> {
    Ok(get_cached_insight_internal(&repo, &app_handle).await)
}

/// 批量检查仓库是否已有本地洞察
#[tauri::command]
pub async fn check_insights_batch(
    repos: Vec<RepoInfo>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let mut exists = Vec::new();
    for repo in repos {
        if let Some(path) = get_cache_path(&repo, &app_handle).await {
            if path.exists() {
                exists.push(repo.url);
            }
        }
    }
    Ok(exists)
}

async fn get_cached_insight_internal(repo: &RepoInfo, app_handle: &tauri::AppHandle) -> Option<String> {
    let cache_path = get_cache_path(repo, app_handle).await?;
    if cache_path.exists() {
        fs::read_to_string(cache_path).ok()
    } else {
        None
    }
}

async fn save_cache(repo: &RepoInfo, content: &str, app_handle: &tauri::AppHandle) {
    let trimmed_content = content.trim();
    if trimmed_content.is_empty() || trimmed_content.len() < 10 {
        return; // 不缓存过短或空的内容
    }

    if let Some(cache_path) = get_cache_path(repo, app_handle).await {
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&cache_path, content) {
            eprintln!("Failed to save cache to {:?}: {}", cache_path, e);
        }
    }
}

async fn get_cache_path(repo: &RepoInfo, app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    let mut path = app_handle.path().app_data_dir().ok()?;
    path.push("ai_insights");
    
    // 清理并标准化文件名以避免特殊字符或大小写带来的不匹配
    let author_clean = sanitize_filename(&repo.author);
    let name_clean = sanitize_filename(&repo.name);
    
    path.push(format!("{}_{}.md", author_clean, name_clean));
    Some(path)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

/// 使用直接提供的 API Key 进行总结（向后兼容）
async fn summarize_with_api_key(
    api_key: String,
    messages: Vec<ChatMessage>,
    on_event: Channel<StreamPayload>,
) -> Result<(), String> {
    // 创建临时的 OpenAI 配置
    use crate::models::{ModelConfig, ModelProvider};

    let config = ModelConfig::new(
        "临时 OpenAI 配置".to_string(),
        ModelProvider::OpenAI,
        ModelProvider::OpenAI.default_api_base_url(),
        api_key,
        ModelProvider::OpenAI.default_model_name(),
    );

    // 创建 LLM 提供商
    let provider = LLMFactory::create_provider(&config)
        .map_err(|e| e.to_string())?;

    // 执行聊天补全（流式）
    let response = provider.chat_completion(messages, &config.default_model, true)
        .await
        .map_err(|e| e.to_string())?;

    match response {
        LLMResponse::Completion { content, .. } => {
            let _ = on_event.send(StreamPayload::Token(content));
            let _ = on_event.send(StreamPayload::Done);
            Ok(())
        }
        LLMResponse::Stream { mut stream } => {
            while let Some(chunk) = stream.recv().await {
                match chunk {
                    StreamChunk::Text(text) => {
                        let _ = on_event.send(StreamPayload::Token(text));
                    }
                    StreamChunk::Error(err) => {
                        let _ = on_event.send(StreamPayload::Error(err));
                        return Err("流式响应错误".to_string());
                    }
                    StreamChunk::Done => {
                        let _ = on_event.send(StreamPayload::Done);
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}

/// 测试模型连接
#[tauri::command]
pub async fn test_model_connection(
    model_config_id: String,
    config_manager: tauri::State<'_, ConfigManagerState>,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    let configs = manager.get_all_model_configs().await.map_err(|e| e.to_string())?;
    let config = configs.iter().find(|c| c.id == model_config_id).ok_or_else(|| format!("找不到模型配置: {}", model_config_id))?;
    let provider = LLMFactory::create_provider(config).map_err(|e| e.to_string())?;
    provider.test_connection().await.map_err(|e| e.to_string())
}

/// 获取模型列表
#[tauri::command]
pub async fn list_models(
    model_config_id: String,
    config_manager: tauri::State<'_, ConfigManagerState>,
) -> Result<Vec<crate::models::ModelInfo>, String> {
    let manager = config_manager.lock().await;
    let configs = manager.get_all_model_configs().await.map_err(|e| e.to_string())?;
    let config = configs.iter().find(|c| c.id == model_config_id).ok_or_else(|| format!("找不到模型配置: {}", model_config_id))?;
    let provider = LLMFactory::create_provider(config).map_err(|e| e.to_string())?;
    provider.list_models().await.map_err(|e| e.to_string())
}

/// 获取 GitHub 仓库的文件树结构
async fn fetch_tree(author: &str, name: &str) -> Option<String> {
    let client = reqwest::Client::builder().user_agent("github-capture").build().ok()?;
    
    // 我们先尝试获取默认分支的 1 层深度目录
    let url = format!("https://api.github.com/repos/{}/{}/contents/", author, name);
    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(items) = resp.json::<Vec<serde_json::Value>>().await {
            let mut tree = String::new();
            for (i, item) in items.iter().take(50).enumerate() {
                let name = item["name"].as_str().unwrap_or("");
                let kind = if item["type"] == "dir" { "[DIR]" } else { "[FILE]" };
                tree.push_str(&format!("{} {}\n", kind, name));
                if i >= 49 { tree.push_str("... (已省略更多文件)"); }
            }
            return Some(tree);
        }
    }
    None
}

/// 获取单个文件的原始内容，可选限制长度
async fn fetch_file_content(author: &str, name: &str, path: &str, limit: Option<usize>) -> Option<String> {
    let client = reqwest::Client::builder().user_agent("github-capture").build().ok()?;
    let urls = [
        format!("https://raw.githubusercontent.com/{}/{}/refs/heads/main/{}", author, name, path),
        format!("https://raw.githubusercontent.com/{}/{}/refs/heads/master/{}", author, name, path),
    ];

    for url in urls {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    return Some(match limit {
                        Some(l) => text.chars().take(l).collect(),
                        None => text
                    });
                }
            }
        }
    }
    None
}

/// 尝试获取 GitHub 仓库的 README 内容
async fn fetch_readme_with_limit(author: &str, name: &str, limit: Option<usize>) -> Option<String> {
    fetch_file_content(author, name, "README.md", limit).await
}

async fn fetch_readme(author: &str, name: &str) -> Option<String> {
    fetch_readme_with_limit(author, name, Some(1500)).await
}