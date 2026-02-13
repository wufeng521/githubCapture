//! Azure OpenAI 提供商实现

use crate::models::{ModelConfig, ModelInfo, ChatMessage, ModelProvider};
use super::{LLMProvider, LLMError, LLMResponse};

/// Azure OpenAI 提供商
pub struct AzureOpenAIProvider {
    config: ModelConfig,
}

impl AzureOpenAIProvider {
    /// 创建新的 Azure OpenAI 提供商实例
    pub fn new(config: &ModelConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for AzureOpenAIProvider {
    async fn chat_completion(
        &self,
        _messages: Vec<ChatMessage>,
        _model: &str,
        _stream: bool,
    ) -> Result<LLMResponse, LLMError> {
        // TODO: 实现 Azure OpenAI API 调用
        // Azure OpenAI 与 OpenAI API 类似，但端点格式不同
        Err(LLMError::ConfigurationError(
            "Azure OpenAI provider not yet implemented".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        // TODO: 实现 Azure OpenAI 模型列表获取
        // Azure OpenAI 需要通过管理 API 获取模型列表
        Ok(vec![
            ModelInfo {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: ModelProvider::AzureOpenAI,
                context_length: Some(8192),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                provider: ModelProvider::AzureOpenAI,
                context_length: Some(128000),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
            ModelInfo {
                id: "gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                provider: ModelProvider::AzureOpenAI,
                context_length: Some(16385),
                max_tokens: Some(4096),
                supports_streaming: true,
                supports_function_calling: true,
            },
        ])
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        // TODO: 实现 Azure OpenAI 连接测试
        Err(LLMError::ConfigurationError(
            "Azure OpenAI connection test not yet implemented".to_string(),
        ))
    }
}