use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendingRepo {
    pub author: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub stars: String,
    pub forks: String,
    pub stars_today: String,
    pub url: String,
    pub topic: String,
}

fn get_topic(name: &str, desc: &str) -> String {
    let content = format!("{} {}", name, desc).to_lowercase();
    
    if content.contains("ai") || content.contains("llm") || content.contains("gpt") || 
       content.contains("model") || content.contains("inference") || content.contains("agent") ||
       content.contains("rag") || content.contains("learning") || content.contains("llama") {
        return "AI / LLM".to_string();
    }
    
    if content.contains("web") || content.contains("react") || content.contains("vue") || 
       content.contains("frontend") || content.contains("backend") || content.contains("nextjs") ||
       content.contains("api") || content.contains("framework") {
        return "Web / App".to_string();
    }

    if content.contains("cli") || content.contains("tool") || content.contains("utility") || 
       content.contains("helper") || content.contains("automation") || content.contains("workflow") {
        return "Tools / CLI".to_string();
    }

    if content.contains("system") || content.contains("kernel") || content.contains("driver") || 
       content.contains("hardware") || content.contains("linux") || content.contains("os") ||
       content.contains("memory") || content.contains("cpu") {
        return "Systems / OS".to_string();
    }

    if content.contains("ios") || content.contains("android") || content.contains("mobile") || 
       content.contains("flutter") || content.contains("swift") || content.contains("kotlin") {
        return "Mobile".to_string();
    }

    "General".to_string()
}

fn parse_github_number(s: &str) -> u64 {
    s.chars()
        .filter(|c| c.is_digit(10))
        .collect::<String>()
        .parse::<u64>()
        .unwrap_or(0)
}

#[tauri::command]
pub async fn get_trending(language: Option<String>, since: String) -> Result<Vec<TrendingRepo>, String> {
    fetch_trending(language, &since).await
}

pub async fn fetch_trending(language: Option<String>, since: &str) -> Result<Vec<TrendingRepo>, String> {
    let url = match language {
        Some(lang) => format!("https://github.com/trending/{}?since={}", lang, since),
        None => format!("https://github.com/trending?since={}", since),
    };

    let response = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let document = Html::parse_document(&response);
    let repo_selector = Selector::parse("article.Box-row").unwrap();
    let title_selector = Selector::parse("h2 a").unwrap();
    let desc_selector = Selector::parse("p.col-9").unwrap();
    let meta_selector = Selector::parse("div.f6.color-fg-muted").unwrap();
    let lang_selector = Selector::parse("span[itemprop='programmingLanguage']").unwrap();
    let stars_selector = Selector::parse("a.Link--muted:nth-of-type(1)").unwrap();
    let forks_selector = Selector::parse("a.Link--muted:nth-of-type(2)").unwrap();
    let stars_today_selector = Selector::parse("span.float-sm-right").unwrap();

    let mut repos = Vec::new();

    for repo_node in document.select(&repo_selector) {
        let title_link = match repo_node.select(&title_selector).next() {
            Some(link) => link,
            None => continue, // 跳过无效节点
        };
        let full_name = title_link.text().collect::<Vec<_>>().join("");
        let parts: Vec<&str> = full_name.split('/').map(|s| s.trim()).collect();
        
        let author = parts.get(0).unwrap_or(&"").to_string();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let url = format!("https://github.com{}", title_link.value().attr("href").unwrap_or(""));

        let description = repo_node.select(&desc_selector)
            .next()
            .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let meta_node = repo_node.select(&meta_selector).next();
        
        let language = meta_node.and_then(|m| m.select(&lang_selector).next())
            .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let stars = meta_node.and_then(|m| m.select(&stars_selector).next())
            .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let forks = meta_node.and_then(|m| m.select(&forks_selector).next())
            .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let stars_today = meta_node.and_then(|m| m.select(&stars_today_selector).next())
            .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let topic = get_topic(&name, &description);
        
        repos.push(TrendingRepo {
            author,
            name,
            description,
            language,
            stars,
            forks,
            stars_today,
            url,
            topic,
        });
    }

    // 排序逻辑：根据 stars_today (增速) 降序排，相同增速则按 stars (总量) 降序排
    repos.sort_by(|a, b| {
        let a_today = parse_github_number(&a.stars_today);
        let b_today = parse_github_number(&b.stars_today);
        
        let a_total = parse_github_number(&a.stars);
        let b_total = parse_github_number(&b.stars);

        // 优先比较增速，其次比较总量
        b_today.cmp(&a_today).then_with(|| b_total.cmp(&a_total))
    });

    Ok(repos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_trending() {
        let result = fetch_trending(None, "daily").await;
        assert!(result.is_ok());
        let repos = result.unwrap();
        assert!(!repos.is_empty());
        println!("Fetched {} repos", repos.len());
        println!("First repo: {:?}", repos[0]);
    }
}
