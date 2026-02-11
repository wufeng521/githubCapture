use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub repo: TrendingRepo,
}

#[tauri::command]
pub async fn smart_search(query: String, api_key: String) -> Result<Vec<TrendingRepo>, String> {
    // 1. 调用 LLM 解析意图 (简易版实现，未来可扩展为完整 Prompt)
    // 这里我们先实现基础的 GitHub Search 调用，
    // 如果 query 看起来像口语，可以先通过 LLM 转义。
    
    let github_query = if query.len() > 20 || query.contains(' ') {
        parse_intent_with_llm(&query, &api_key).await.unwrap_or(query)
    } else {
        query
    };

    search_github_repositories(&github_query).await
}

async fn parse_intent_with_llm(user_input: &str, api_key: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let prompt = format!(
        "Translate the following user intent into a GitHub search query string. \
        Only return the query string, no explanation. \
        Example: 'beginner friendly rust ai' -> 'topic:ai language:rust stars:<5000' \
        Intent: '{}'", 
        user_input
    );

    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.3
    });

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = res.json().await.ok()?;
    let content = json["choices"][0]["message"]["content"].as_str()?.trim().to_string();
    
    Some(content)
}

async fn search_github_repositories(query: &str) -> Result<Vec<TrendingRepo>, String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("github-capture-app"));
    
    let url = format!("https://api.github.com/search/repositories?q={}&sort=stars&order=desc", urlencoding::encode(query));

    let res = client.get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("GitHub API Error: {}", res.status()));
    }

    let search_res: GithubSearchResponse = res.json().await.map_err(|e| e.to_string())?;

    let repos = search_res.items.into_iter().map(|item| {
        let parts: Vec<&str> = item.full_name.split('/').collect();
        TrendingRepo {
            author: parts.get(0).unwrap_or(&"").to_string(),
            name: parts.get(1).unwrap_or(&"").to_string(),
            description: item.description.unwrap_or_default(),
            language: item.language.unwrap_or_else(|| "Unknown".to_string()),
            stars: format_number(item.stargazers_count),
            forks: format_number(item.forks_count),
            stars_today: "".to_string(), // 搜索结果不直接提供今日星数增量
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
