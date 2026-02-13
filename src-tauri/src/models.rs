use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

/// 模型提供商枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelProvider {
    OpenAI,      // OpenAI API (chat completions)
    Anthropic,   // Claude API
    Google,      // Gemini API
    DeepSeek,    // DeepSeek API
    AzureOpenAI, // Azure OpenAI
    Custom(String), // 支持自定义厂商（OpenAI兼容）
}

impl Default for ModelProvider {
    fn default() -> Self {
        ModelProvider::OpenAI
    }
}

impl ModelProvider {
    /// 获取提供商的显示名称
    pub fn display_name(&self) -> String {
        match self {
            ModelProvider::OpenAI => "OpenAI".to_string(),
            ModelProvider::Anthropic => "Anthropic (Claude)".to_string(),
            ModelProvider::Google => "Google (Gemini)".to_string(),
            ModelProvider::DeepSeek => "DeepSeek".to_string(),
            ModelProvider::AzureOpenAI => "Azure OpenAI".to_string(),
            ModelProvider::Custom(name) => format!("Custom ({})", name),
        }
    }

    /// 获取默认的API基础URL
    pub fn default_api_base_url(&self) -> String {
        match self {
            ModelProvider::OpenAI => "https://api.openai.com/v1".to_string(),
            ModelProvider::Anthropic => "https://api.anthropic.com".to_string(),
            ModelProvider::Google => "https://generativelanguage.googleapis.com/v1".to_string(),
            ModelProvider::DeepSeek => "https://api.deepseek.com".to_string(),
            ModelProvider::AzureOpenAI => "".to_string(), // 必须由用户配置
            ModelProvider::Custom(_) => "".to_string(), // 必须由用户配置
        }
    }

    /// 获取默认的模型名称
    pub fn default_model_name(&self) -> String {
        match self {
            ModelProvider::OpenAI => "gpt-4o-mini".to_string(),
            ModelProvider::Anthropic => "claude-3-haiku-20240307".to_string(),
            ModelProvider::Google => "gemini-pro".to_string(),
            ModelProvider::DeepSeek => "deepseek-chat".to_string(),
            ModelProvider::AzureOpenAI => "gpt-4".to_string(),
            ModelProvider::Custom(_) => "custom-model".to_string(),
        }
    }

    /// 检查该提供商是否需要API密钥
    pub fn requires_api_key(&self) -> bool {
        match self {
            ModelProvider::OpenAI => true,
            ModelProvider::Anthropic => true,
            ModelProvider::Google => true,
            ModelProvider::DeepSeek => true,
            ModelProvider::AzureOpenAI => true,
            ModelProvider::Custom(_) => true,
        }
    }

    /// 检查该提供商是否需要自定义API基础URL
    pub fn requires_custom_base_url(&self) -> bool {
        match self {
            ModelProvider::AzureOpenAI => true,
            ModelProvider::Custom(_) => true,
            _ => false,
        }
    }
}

/// 模型配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_id")]
    pub id: String, // 唯一标识符（UUID）
    pub name: String, // 显示名称
    pub provider: ModelProvider,
    pub api_base_url: String, // API基础URL
    pub api_key: String, // API密钥（加密存储）
    pub default_model: String, // 默认模型名称
    pub enabled: bool,
    #[serde(default = "default_now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "default_now")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_id() -> String { Uuid::new_v4().to_string() }
fn default_now() -> chrono::DateTime<chrono::Utc> { Utc::now() }

impl ModelConfig {
    /// 创建一个新的模型配置
    pub fn new(
        name: String,
        provider: ModelProvider,
        api_base_url: String,
        api_key: String,
        default_model: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            provider,
            api_base_url,
            api_key,
            default_model,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建一个默认的OpenAI配置
    pub fn default_openai(api_key: String) -> Self {
        Self::new(
            "OpenAI (默认)".to_string(),
            ModelProvider::OpenAI,
            ModelProvider::OpenAI.default_api_base_url(),
            api_key,
            ModelProvider::OpenAI.default_model_name(),
        )
    }

    /// 更新配置
    pub fn update(&mut self, updates: ModelConfigUpdate) {
        if let Some(name) = updates.name {
            self.name = name;
        }
        if let Some(provider) = updates.provider {
            self.provider = provider;
        }
        if let Some(api_base_url) = updates.api_base_url {
            self.api_base_url = api_base_url;
        }
        if let Some(api_key) = updates.api_key {
            self.api_key = api_key;
        }
        if let Some(default_model) = updates.default_model {
            self.default_model = default_model;
        }
        if let Some(enabled) = updates.enabled {
            self.enabled = enabled;
        }
        self.updated_at = chrono::Utc::now();
    }
}

/// 模型配置更新结构（用于部分更新）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigUpdate {
    pub name: Option<String>,
    pub provider: Option<ModelProvider>,
    pub api_base_url: Option<String>,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    pub enabled: Option<bool>,
}

/// 模型信息（从API拉取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: ModelProvider,
    pub context_length: Option<u32>,
    pub max_tokens: Option<u32>,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub active_model_config_id: Option<String>, // 当前激活的模型配置ID
    pub model_configs: Vec<ModelConfig>, // 所有模型配置
    pub model_cache: HashMap<String, Vec<ModelInfo>>, // 模型列表缓存（按提供商）
    pub cache_expires_at: Option<chrono::DateTime<chrono::Utc>>, // 缓存过期时间
}

impl AppConfig {
    /// 获取当前激活的模型配置
    pub fn get_active_config(&self) -> Option<&ModelConfig> {
        self.active_model_config_id
            .as_ref()
            .and_then(|id| self.model_configs.iter().find(|config| config.id == *id))
    }

    /// 通过ID查找模型配置
    pub fn get_config_by_id(&self, id: &str) -> Option<&ModelConfig> {
        self.model_configs.iter().find(|config| config.id == id)
    }

    /// 添加新的模型配置
    pub fn add_config(&mut self, config: ModelConfig) {
        self.model_configs.push(config);
        self.updated();
    }

    /// 更新现有模型配置
    pub fn update_config(&mut self, id: &str, updates: ModelConfigUpdate) -> bool {
        if let Some(config) = self.model_configs.iter_mut().find(|c| c.id == id) {
            config.update(updates);
            self.updated();
            true
        } else {
            false
        }
    }

    /// 删除模型配置
    pub fn remove_config(&mut self, id: &str) -> bool {
        let original_len = self.model_configs.len();
        self.model_configs.retain(|config| config.id != id);

        let removed = self.model_configs.len() < original_len;
        if removed {
            // 如果删除的是激活配置，清除激活ID
            if self.active_model_config_id.as_ref() == Some(&id.to_string()) {
                self.active_model_config_id = None;
            }
            self.updated();
        }
        removed
    }

    /// 设置激活的模型配置
    pub fn set_active_config(&mut self, id: &str) -> bool {
        if self.model_configs.iter().any(|config| config.id == id) {
            self.active_model_config_id = Some(id.to_string());
            self.updated();
            true
        } else {
            false
        }
    }

    /// 检查是否有任何启用的模型配置
    pub fn has_enabled_configs(&self) -> bool {
        self.model_configs.iter().any(|config| config.enabled)
    }

    /// 获取所有启用的模型配置
    pub fn get_enabled_configs(&self) -> Vec<&ModelConfig> {
        self.model_configs.iter().filter(|c| c.enabled).collect()
    }

    /// 获取指定提供商的所有配置
    pub fn get_configs_by_provider(&self, provider: &ModelProvider) -> Vec<&ModelConfig> {
        self.model_configs
            .iter()
            .filter(|c| &c.provider == provider)
            .collect()
    }

    /// 更新缓存过期时间
    pub fn update_cache_expiry(&mut self, hours: i64) {
        self.cache_expires_at = Some(chrono::Utc::now() + chrono::Duration::hours(hours));
    }

    /// 检查缓存是否过期
    pub fn is_cache_expired(&self) -> bool {
        match self.cache_expires_at {
            Some(expiry) => chrono::Utc::now() >= expiry,
            None => true,
        }
    }

    /// 标记配置已更新
    fn updated(&mut self) {
        // 可以在这里添加持久化逻辑
    }
}

/// 聊天消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self::new("system", content)
    }

    pub fn user(content: &str) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: &str) -> Self {
        Self::new("assistant", content)
    }
}