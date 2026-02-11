use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use futures_util::StreamExt;
use reqwest_eventsource::{Event, EventSource};

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoInfo {
    pub author: String,
    pub name: String,
    pub description: String,
    pub language: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamPayload {
    Token(String),
    Error(String),
    Done,
}

#[tauri::command]
pub async fn summarize_repo(
    repo: RepoInfo,
    api_key: String,
    on_event: Channel<StreamPayload>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    let prompt = format!(
        "请对以下 GitHub 项目进行深入浅出的深度总结：\n项目：{}/{}\n描述：{}\n语言：{}\n\n请包含以下维度：\n1. 核心技术架构\n2. 解决了什么核心痛点\n3. 适合谁用以及如何快速上手（3句话以内）\n请使用 Markdown 格式。",
        repo.author, repo.name, repo.description, repo.language
    );

    let payload = serde_json::json!({
        "model": "gpt-4o-mini", // 使用更轻快的模型
        "messages": [
            {
                "role": "system",
                "content": "你是一个资深的软件架构师和技术布道者，擅长简明扼要地总结技术项目。"
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": true
    });

    let mut source = EventSource::new(
        client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
    ).map_err(|e| e.to_string())?;

    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Message(message)) => {
                if message.data == "[DONE]" {
                    let _ = on_event.send(StreamPayload::Done);
                    break;
                }

                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&message.data) {
                    if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                        let _ = on_event.send(StreamPayload::Token(content.to_string()));
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                let _ = on_event.send(StreamPayload::Error(e.to_string()));
                return Err(e.to_string());
            }
        }
    }

    Ok(())
}
