//! Google (Gemini) 提供商实现

use crate::models::{ModelConfig, ModelInfo, ChatMessage, ModelProvider};
use super::{LLMProvider, LLMError, LLMResponse};

/// Google 提供商
pub struct GoogleProvider {
    config: ModelConfig,
}

impl GoogleProvider {
    /// 创建新的 Google 提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for GoogleProvider {
    async fn chat_completion(
        &self,
        _messages: Vec<ChatMessage>,
        _model: &str,
        _stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        // TODO: 实现 Google Gemini API 调用
        Err(LLMError::ConfigurationError(
            "Google provider not yet implemented".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        // TODO: 实现 Google 模型列表获取
        Ok(vec![
            ModelInfo {
                id: "gemini-pro".to_string(),
                name: "Gemini Pro".to_string(),
                provider: ModelProvider::Google,
                context_length: Some(30720),
                max_tokens: Some(2048),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "gemini-pro-vision".to_string(),
                name: "Gemini Pro Vision".to_string(),
                provider: ModelProvider::Google,
                context_length: Some(12288),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "gemini-1.5-pro".to_string(),
                name: "Gemini 1.5 Pro".to_string(),
                provider: ModelProvider::Google,
                context_length: Some(1000000),
                max_tokens: Some(8192),
                supports_streaming: true,
                supports_function_calling: true,
            },
        ])
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        // TODO: 实现 Google 连接测试
        Err(LLMError::ConfigurationError(
            "Google connection test not yet implemented".to_string(),
        ))
    }
}