//! LLM 抽象层
//!
//! 提供统一的 LLM 接口，支持多种模型厂商。

use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::models::{ModelConfig, ModelInfo, ChatMessage};

// 导出各个厂商实现
pub mod openai;
pub mod anthropic;
pub mod google;
pub mod deepseek;
pub mod azure_openai;
pub mod custom;

/// LLM 提供商的统一接口
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    /// 执行聊天补全
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        stream: bool,
    ) -> Result<LLMResponse, LLMError>;

    /// 列出可用的模型
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError>;

    /// 测试连接和认证
    async fn test_connection(&self) -> Result<(), LLMError>;
}

/// LLM 响应类型
#[derive(Debug)]
pub enum LLMResponse {
    /// 非流式响应
    Completion {
        content: String,
        model: String,
        usage: Option<Usage>,
    },
    /// 流式响应通道
    Stream {
        stream: tokio::sync::mpsc::Receiver<StreamChunk>,
    },
}

/// 流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    /// 文本块
    Text(String),
    /// 错误
    Error(String),
    /// 完成
    Done,
}

/// 使用量统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 统一的 LLM 错误类型
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API请求失败: {0}")]
    RequestFailed(String),
    #[error("认证失败: {0}")]
    AuthenticationFailed(String),
    #[error("模型不可用: {0}")]
    ModelUnavailable(String),
    #[error("额度不足")]
    InsufficientQuota,
    #[error("网络错误: {0}")]
    NetworkError(String),
    #[error("配置错误: {0}")]
    ConfigurationError(String),
    #[error("解析错误: {0}")]
    ParseError(String),
    #[error("未知错误: {0}")]
    Unknown(String),
}

impl LLMError {
    /// 从 HTTP 状态码创建错误
    pub fn from_status_code(status: u16, message: &str) -> Self {
        match status {
            401 | 403 => LLMError::AuthenticationFailed(message.to_string()),
            404 => LLMError::ModelUnavailable(message.to_string()),
            429 => LLMError::InsufficientQuota,
            400..=499 => LLMError::RequestFailed(message.to_string()),
            500..=599 => LLMError::NetworkError(message.to_string()),
            _ => LLMError::Unknown(message.to_string()),
        }
    }
}

/// LLM 提供商工厂
pub struct LLMFactory;

impl LLMFactory {
    /// 从模型配置创建 LLM 提供商实例
    pub fn create_provider(config: &ModelConfig) -> Result<Box<dyn LLMProvider>, LLMError> {
        match config.provider {
            crate::models::ModelProvider::OpenAI => {
                Ok(Box::new(openai::OpenAIProvider::new(config)))
            }
            crate::models::ModelProvider::Anthropic => {
                Ok(Box::new(anthropic::AnthropicProvider::new(config)))
            }
            crate::models::ModelProvider::Google => {
                Ok(Box::new(google::GoogleProvider::new(config)))
            }
            crate::models::ModelProvider::DeepSeek => {
                Ok(Box::new(deepseek::DeepSeekProvider::new(config)))
            }
            crate::models::ModelProvider::AzureOpenAI => {
                Ok(Box::new(azure_openai::AzureOpenAIProvider::new(config)))
            }
            crate::models::ModelProvider::Custom(_) => {
                Ok(Box::new(custom::CustomProvider::new(config)))
            }
        }
    }

    /// 获取所有支持的提供商类型
    pub fn supported_providers() -> Vec<crate::models::ModelProvider> {
        vec![
            crate::models::ModelProvider::OpenAI,
            crate::models::ModelProvider::Anthropic,
            crate::models::ModelProvider::Google,
            crate::models::ModelProvider::DeepSeek,
            crate::models::ModelProvider::AzureOpenAI,
            crate::models::ModelProvider::Custom("Custom".to_string()),
        ]
    }
}

/// 为 LLMError 实现 From trait，便于错误转换
impl From<reqwest::Error> for LLMError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_status() {
            if let Some(status) = err.status() {
                LLMError::from_status_code(status.as_u16(), &err.to_string())
            } else {
                LLMError::NetworkError(err.to_string())
            }
        } else if err.is_connect() || err.is_timeout() {
            LLMError::NetworkError(err.to_string())
        } else {
            LLMError::Unknown(err.to_string())
        }
    }
}

impl From<serde_json::Error> for LLMError {
    fn from(err: serde_json::Error) -> Self {
        LLMError::ParseError(err.to_string())
    }
}

/// 为异步 trait 启用 async_trait 宏
#[allow(unused_imports)]
use async_trait::async_trait;