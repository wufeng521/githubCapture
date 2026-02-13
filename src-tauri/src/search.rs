use serde::Deserialize;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use crate::trending::TrendingRepo;
use crate::models::ChatMessage;
use crate::llm::LLMFactory;
use crate::config::commands::ConfigManagerState;

#[derive(Debug, Deserialize)]
struct GithubSearchResponse {
    items: Vec<GithubRepoItem>,
}

#[derive(Debug, Deserialize)]
struct GithubRepoItem {
    full_name: String,
    description: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    language: Option<String>,
    html_url: String,
    topics: Option<Vec<String>>,
    pushed_at: Option<String>,
    license: Option<GithubLicense>,
}

#[derive(Debug, Deserialize)]
struct GithubLicense {
    name: String,
}

/// AI 改写用户查询：将口语化意图转换为 GitHub 搜索语法
///
/// 支持两种模式：
/// 1. 旧模式：提供 api_key 参数，使用指定的 API Key
/// 2. 新模式：提供 model_config_id 参数，使用配置管理器中的模型配置
#[tauri::command]
pub async fn ai_rewrite_query(
    query: String,
    api_key: Option<String>,
    model_config_id: Option<String>,
    config_manager: tauri::State<'_, ConfigManagerState>,
) -> Result<String, String> {
    let prompt = format!(
        "You are a GitHub search query optimizer. Convert the following natural language intent \
        into a precise GitHub search query string using qualifiers like language:, topic:, stars:, pushed:, etc.\n\
        Rules:\n\
        - Only return the query string, nothing else\n\
        - Keep it concise but precise\n\
        - Use appropriate qualifiers based on the intent\n\
        - If language is mentioned, use language: qualifier\n\
        - If popularity is implied, use stars: qualifier\n\
        - If recency matters, use pushed: qualifier\n\n\
        Examples:\n\
        Input: '适合初学者的 Rust AI 项目' -> 'language:rust topic:ai good-first-issues:>0 stars:>100'\n\
        Input: '最近很火的 React UI 组件库' -> 'language:typescript topic:react topic:ui stars:>1000 pushed:>2025-01-01'\n\
        Input: 'golang web framework' -> 'language:go topic:web-framework stars:>500'\n\n\
        Intent: '{}'",
        query
    );

    let messages = vec![ChatMessage::user(&prompt)];

    // 确定使用哪种模式
    let result = if let Some(config_id) = model_config_id {
        // 新模式：使用配置管理器
        rewrite_with_config(config_id, messages, &config_manager).await
    } else if let Some(api_key) = api_key {
        // 旧模式：使用直接提供的 API Key
        rewrite_with_api_key(api_key, messages).await
    } else {
        return Err("必须提供 API Key 或模型配置 ID".to_string());
    };

    result
}

/// 使用配置管理器中的模型配置进行查询改写
async fn rewrite_with_config(
    config_id: String,
    messages: Vec<ChatMessage>,
    config_manager: &tauri::State<'_, ConfigManagerState>,
) -> Result<String, String> {
    let manager_lock = config_manager.lock().await;

    // 获取模型配置
    let configs = manager_lock.get_all_model_configs().await
        .map_err(|e| e.to_string())?;

    let config = configs.iter()
        .find(|c| c.id == config_id)
        .ok_or_else(|| format!("找不到模型配置: {}", config_id))?;

    // 创建 LLM 提供商
    let provider = LLMFactory::create_provider(config)
        .map_err(|e| e.to_string())?;

    // 执行聊天补全（非流式）
    let response = provider.chat_completion(messages, &config.default_model, false)
        .await
        .map_err(|e| e.to_string())?;

    match response {
        crate::llm::LLMResponse::Completion { content, .. } => {
            Ok(content.trim().to_string())
        }
        crate::llm::LLMResponse::Stream { .. } => {
            Err("预期非流式响应，但收到流式响应".to_string())
        }
    }
}

/// 使用直接提供的 API Key 进行查询改写（向后兼容）
async fn rewrite_with_api_key(
    api_key: String,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API Key 未配置，请在设置中填写".to_string());
    }

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

    // 执行聊天补全（非流式）
    let response = provider.chat_completion(messages, &config.default_model, false)
        .await
        .map_err(|e| e.to_string())?;

    match response {
        crate::llm::LLMResponse::Completion { content, .. } => {
            Ok(content.trim().to_string())
        }
        crate::llm::LLMResponse::Stream { .. } => {
            Err("预期非流式响应，但收到流式响应".to_string())
        }
    }
}

/// 直接搜索 GitHub 仓库（不经过 AI 改写）
#[tauri::command]
pub async fn search_github(query: String) -> Result<Vec<TrendingRepo>, String> {
    search_github_repositories(&query).await
}

async fn search_github_repositories(query: &str) -> Result<Vec<TrendingRepo>, String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("github-capture-app"));

    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=20",
        urlencoding::encode(query)
    );

    let res = client.get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("GitHub API 请求失败: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("GitHub API 错误: {}", res.status()));
    }

    let search_res: GithubSearchResponse = res.json().await.map_err(|e| format!("解析失败: {}", e))?;

    let repos = search_res.items.into_iter().map(|item| {
        let parts: Vec<&str> = item.full_name.split('/').collect();
        TrendingRepo {
            author: parts.get(0).unwrap_or(&"").to_string(),
            name: parts.get(1).unwrap_or(&"").to_string(),
            description: item.description.unwrap_or_default(),
            language: item.language.unwrap_or_else(|| "Unknown".to_string()),
            stars: format_number(item.stargazers_count),
            forks: format_number(item.forks_count),
            stars_today: "".to_string(),
            url: item.html_url,
            topic: "Search Result".to_string(),
            built_by: Vec::new(),
            topics: item.topics.unwrap_or_default(),
            pushed_at: item.pushed_at.unwrap_or_default(),
            license: item.license.map(|l| l.name).unwrap_or_else(|| "None".to_string()),
        }
    }).collect();

    Ok(repos)
}

fn format_number(num: u64) -> String {
    if num >= 1000 {
        format!("{:.1}k", num as f64 / 1000.0)
    } else {
        num.to_string()
    }
}