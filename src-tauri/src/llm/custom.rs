//! 自定义提供商实现（OpenAI兼容）
//!
//! 支持任意 OpenAI 兼容 API（如 Ollama, vLLM, LiteLLM, Together AI 等）。

use crate::models::{ModelConfig, ModelInfo, ChatMessage};
use super::{LLMProvider, LLMError, LLMResponse};
use super::openai::OpenAIProvider;

/// 自定义提供商（基于 OpenAI 兼容协议）
pub struct CustomProvider {
    /// 内部使用 OpenAI 提供商处理请求
    inner: OpenAIProvider,
    config: ModelConfig,
}

impl CustomProvider {
    /// 创建新的自定义提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            inner: OpenAIProvider::new(config),
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for CustomProvider {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        // OpenAI 兼容协议，直接委托
        self.inner.chat_completion(messages, model, stream).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        // 尝试从 API 获取模型列表
        match self.inner.list_models().await {
            Ok(mut models) => {
                // 修正 provider 标记
                for model in &mut models {
                    model.provider = self.config.provider.clone();
                }
                Ok(models)
            }
            Err(_) => {
                // 返回默认的模型信息
                Ok(vec![
                    ModelInfo {
                        id: self.config.default_model.clone(),
                        name: self.config.default_model.clone(),
                        provider: self.config.provider.clone(),
                        context_length: None,
                        max_tokens: None,
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