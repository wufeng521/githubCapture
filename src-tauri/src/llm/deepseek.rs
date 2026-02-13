//! DeepSeek 提供商实现
//!
//! DeepSeek API 完全兼容 OpenAI 格式，复用 OpenAI 的请求/响应处理逻辑。

use crate::models::{ModelConfig, ModelInfo, ChatMessage, ModelProvider};
use super::{LLMProvider, LLMError, LLMResponse};
use super::openai::OpenAIProvider;

/// DeepSeek 提供商（基于 OpenAI 兼容协议）
pub struct DeepSeekProvider {
    /// 内部使用 OpenAI 提供商处理请求
    inner: OpenAIProvider,
    config: ModelConfig,
}

impl DeepSeekProvider {
    /// 创建新的 DeepSeek 提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            inner: OpenAIProvider::new(config),
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for DeepSeekProvider {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        // DeepSeek API 完全兼容 OpenAI 格式，直接委托
        self.inner.chat_completion(messages, model, stream).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        // 尝试从 API 获取模型列表，失败则返回预设列表
        match self.inner.list_models().await {
            Ok(mut models) => {
                // 修正 provider 标记
                for model in &mut models {
                    model.provider = ModelProvider::DeepSeek;
                }
                Ok(models)
            }
            Err(_) => {
                // 返回预设的 DeepSeek 模型列表
                Ok(vec![
                    ModelInfo {
                        id: "deepseek-chat".to_string(),
                        name: "DeepSeek Chat (V3)".to_string(),
                        provider: ModelProvider::DeepSeek,
                        context_length: Some(64000),
                        max_tokens: Some(8192),
                        supports_streaming: true,
                        supports_function_calling: true,
                    },
                    ModelInfo {
                        id: "deepseek-reasoner".to_string(),
                        name: "DeepSeek Reasoner (R1)".to_string(),
                        provider: ModelProvider::DeepSeek,
                        context_length: Some(64000),
                        max_tokens: Some(8192),
                        supports_streaming: true,
                        supports_function_calling: false,
                    },
                ])
            }
        }
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        self.inner.test_connection().await
    }
}