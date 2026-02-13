//! OpenAI 提供商实现

use serde_json::json;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::mpsc;
use futures_util::StreamExt;
use crate::models::{ModelConfig, ModelInfo, ChatMessage, ModelProvider};
use super::{LLMProvider, LLMError, LLMResponse, StreamChunk, Usage};

/// OpenAI 提供商
pub struct OpenAIProvider {
    config: ModelConfig,
    client: Client,
}

impl OpenAIProvider {
    /// 创建新的 OpenAI 提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            config: config.clone(),
            client: Client::new(),
        }
    }

    /// 构建 API 端点 URL
    fn build_endpoint_url(&self, path: &str) -> String {
        let base_url = self.config.api_base_url.trim_end_matches('/');
        format!("{}{}", base_url, path)
    }

    /// 处理非流式响应
    async fn handle_completion_response(
        &self,
        response: reqwest::Response,
    ) -> Result<LLMResponse, LLMError> {
        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            let error_msg = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown OpenAI error");
            return Err(LLMError::RequestFailed(error_msg.to_string()));
        }

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| LLMError::ParseError("Missing content in response".to_string()))?
            .to_string();

        let model = json["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let usage = json.get("usage").map(|usage| {
            Usage {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
            }
        });

        Ok(LLMResponse::Completion {
            content,
            model,
            usage,
        })
    }

    /// 处理流式响应
    async fn handle_stream_response(
        &self,
        mut source: EventSource,
    ) -> Result<LLMResponse, LLMError> {
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(event) = source.next().await {
                match event {
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            let _ = tx.send(StreamChunk::Done).await;
                            break;
                        }

                        match serde_json::from_str::<serde_json::Value>(&message.data) {
                            Ok(value) => {
                                if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = tx.send(StreamChunk::Text(content.to_string())).await;
                                    }
                                }
                            }
                            Err(e) => {
                                let chunk = StreamChunk::Error(e.to_string());
                                let _ = tx.send(chunk).await;
                                break;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        let chunk = StreamChunk::Error(e.to_string());
                        let _ = tx.send(chunk).await;
                        break;
                    }
                }
            }

            // 确保发送完成信号
            let _ = tx.send(StreamChunk::Done).await;
        });

        Ok(LLMResponse::Stream { stream: rx })
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        let endpoint = self.build_endpoint_url("/chat/completions");

        // 转换消息格式
        let openai_messages: Vec<serde_json::Value> = messages
            .into_iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect();

        let payload = json!({
            "model": model,
            "messages": openai_messages,
            "stream": stream,
        });

        let request = self.client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload);

        if stream {
            let source = EventSource::new(request)
                .map_err(|e| LLMError::NetworkError(e.to_string()))?;
            self.handle_stream_response(source).await
        } else {
            let response = request
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(LLMError::from_status_code(status.as_u16(), &error_text));
            }

            self.handle_completion_response(response).await
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        let endpoint = self.build_endpoint_url("/models");

        let response = self.client
            .get(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::from_status_code(status.as_u16(), &error_text));
        }

        let json: serde_json::Value = response.json().await?;

        let models = json["data"]
            .as_array()
            .ok_or_else(|| LLMError::ParseError("Invalid models response".to_string()))?
            .iter()
            .filter_map(|model| {
                let id = model["id"].as_str()?.to_string();
                let name = model["id"].as_str()?.to_string();

                // 尝试从能力字段推断支持的功能
                let capabilities = model.get("capabilities").and_then(|c| c.as_object());
                let supports_streaming = true; // OpenAI 所有模型都支持流式
                let supports_function_calling = capabilities
                    .and_then(|c| c.get("function_calling").and_then(|f| f.as_bool()))
                    .unwrap_or(false);

                Some(ModelInfo {
                    id: id.clone(),
                    name,
                    provider: ModelProvider::OpenAI,
                    context_length: model["context_length"].as_u64().map(|n| n as u32),
                    max_tokens: None,
                    supports_streaming,
                    supports_function_calling,
                })
            })
            .collect();

        Ok(models)
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        // 尝试列出模型来测试连接
        let result = self.list_models().await;
        match result {
            Ok(_) => Ok(()),
            Err(LLMError::AuthenticationFailed(msg)) => Err(LLMError::AuthenticationFailed(msg)),
            Err(e) => Err(LLMError::ConfigurationError(format!("Connection test failed: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_endpoint_url() {
        let config = ModelConfig::new(
            "Test".to_string(),
            ModelProvider::OpenAI,
            "https://api.openai.com/v1".to_string(),
            "test-key".to_string(),
            "gpt-4".to_string(),
        );
        let provider = OpenAIProvider::new(&config);

        assert_eq!(
            provider.build_endpoint_url("/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );

        // 测试尾部斜杠处理
        let config2 = ModelConfig::new(
            "Test".to_string(),
            ModelProvider::OpenAI,
            "https://api.openai.com/v1/".to_string(),
            "test-key".to_string(),
            "gpt-4".to_string(),
        );
        let provider2 = OpenAIProvider::new(&config2);
        assert_eq!(
            provider2.build_endpoint_url("/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}