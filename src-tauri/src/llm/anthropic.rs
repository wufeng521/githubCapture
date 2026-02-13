//! Anthropic (Claude) 提供商实现

use crate::models::{ModelConfig, ModelInfo, ChatMessage, ModelProvider};
use super::{LLMProvider, LLMError, LLMResponse};

/// Anthropic 提供商
pub struct AnthropicProvider {
    config: ModelConfig,
}

impl AnthropicProvider {
    /// 创建新的 Anthropic 提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat_completion(
        &self,
        _messages: Vec<ChatMessage>,
        _model: &str,
        _stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        // TODO: 实现 Anthropic API 调用
        Err(LLMError::ConfigurationError(
            "Anthropic provider not yet implemented".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        // TODO: 实现 Anthropic 模型列表获取
        Ok(vec![
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: "Claude 3 Opus".to_string(),
                provider: ModelProvider::Anthropic,
                context_length: Some(200000),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "claude-3-sonnet-20240229".to_string(),
                name: "Claude 3 Sonnet".to_string(),
                provider: ModelProvider::Anthropic,
                context_length: Some(200000),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "claude-3-haiku-20240307".to_string(),
                name: "Claude 3 Haiku".to_string(),
                provider: ModelProvider::Anthropic,
                context_length: Some(200000),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
        ])
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        // TODO: 实现 Anthropic 连接测试
        Err(LLMError::ConfigurationError(
            "Anthropic connection test not yet implemented".to_string(),
        ))
    }
}