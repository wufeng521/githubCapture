use serde::Deserialize;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use crate::trending::TrendingRepo;

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
}

/// AI 改写用户查询：将口语化意图转换为 GitHub 搜索语法
#[tauri::command]
pub async fn ai_rewrite_query(query: String, api_key: String) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API Key 未配置，请在设置中填写".to_string());
    }

    let client = reqwest::Client::new();
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

    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.2,
        "max_tokens": 200
    });

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("OpenAI API 错误: {}", res.status()));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("AI 返回内容为空")?
        .trim()
        .to_string();

    Ok(content)
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
