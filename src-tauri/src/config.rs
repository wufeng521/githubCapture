//! 配置管理系统
//!
//! 负责管理应用配置，包括模型配置的加载、保存、迁移和缓存。

use tauri::AppHandle;
use serde_json::to_value;
use crate::models::{AppConfig, ModelConfig, ModelProvider, ModelInfo, ModelConfigUpdate};

/// 配置管理器
pub struct ConfigManager {
    app_handle: AppHandle,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub async fn new(app_handle: AppHandle) -> Result<Self, ConfigError> {
        let manager = Self { app_handle };

        // 检查是否需要迁移旧配置
        if manager.needs_migration().await? {
            manager.migrate_from_old_format().await?;
        }

        Ok(manager)
    }

    /// 加载应用配置
    pub async fn load_config(&self) -> Result<AppConfig, ConfigError> {
        let store = tauri_plugin_store::StoreBuilder::new(&self.app_handle, "settings.json")
            .build()?;

        let config = if let Some(value) = store.get("app_config") {
            serde_json::from_value(value)?
        } else {
            AppConfig::default()
        };

        Ok(config)
    }

    /// 保存应用配置
    pub async fn save_config(&self, config: &AppConfig) -> Result<(), ConfigError> {
        let store = tauri_plugin_store::StoreBuilder::new(&self.app_handle, "settings.json")
            .build()?;

        let value = to_value(config)?;
        store.set("app_config", value);
        store.save()?;
        Ok(())
    }

    /// 检查是否需要从旧格式迁移
    async fn needs_migration(&self) -> Result<bool, ConfigError> {
        let store = tauri_plugin_store::StoreBuilder::new(&self.app_handle, "settings.json")
            .build()?;

        // 检查是否存在旧格式的 API Key
        let old_key_exists: bool = store.has("openai_api_key");

        // 检查是否存在新格式的配置
        let new_config_exists: bool = store.has("app_config");

        Ok(old_key_exists && !new_config_exists)
    }

    /// 从旧格式迁移配置
    async fn migrate_from_old_format(&self) -> Result<(), ConfigError> {
        log::info!("开始从旧格式迁移配置...");

        let store = tauri_plugin_store::StoreBuilder::new(&self.app_handle, "settings.json").build()?;

        // 加载旧配置
        let old_key: Option<String> = store.get("openai_api_key")
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        if let Some(api_key) = old_key {
            // 创建默认的 OpenAI 配置
            let default_config = ModelConfig::default_openai(api_key);

            // 创建新的应用配置
            let mut new_config = AppConfig::default();
            new_config.add_config(default_config.clone());
            new_config.set_active_config(&default_config.id);

            // 保存新配置
            self.save_config(&new_config).await?;

            log::info!("配置迁移完成，创建了默认的 OpenAI 配置");
        } else {
            log::info!("没有找到旧配置，无需迁移");
        }

        Ok(())
    }

    /// 获取当前激活的模型配置
    pub async fn get_active_model_config(&self) -> Result<Option<ModelConfig>, ConfigError> {
        let config = self.load_config().await?;
        Ok(config.get_active_config().cloned())
    }

    /// 设置激活的模型配置
    pub async fn set_active_model_config(&self, config_id: &str) -> Result<bool, ConfigError> {
        let mut config = self.load_config().await?;
        let success = config.set_active_config(config_id);
        if success {
            self.save_config(&config).await?;
        }
        Ok(success)
    }

    /// 添加新的模型配置
    pub async fn add_model_config(&self, model_config: ModelConfig) -> Result<(), ConfigError> {
        let mut config = self.load_config().await?;
        config.add_config(model_config);
        self.save_config(&config).await
    }

    /// 更新模型配置
    pub async fn update_model_config(
        &self,
        config_id: &str,
        updates: ModelConfigUpdate,
    ) -> Result<bool, ConfigError> {
        let mut config = self.load_config().await?;
        let success = config.update_config(config_id, updates);
        if success {
            self.save_config(&config).await?;
        }
        Ok(success)
    }

    /// 删除模型配置
    pub async fn delete_model_config(&self, config_id: &str) -> Result<bool, ConfigError> {
        let mut config = self.load_config().await?;
        let success = config.remove_config(config_id);
        if success {
            self.save_config(&config).await?;
        }
        Ok(success)
    }

    /// 获取所有模型配置
    pub async fn get_all_model_configs(&self) -> Result<Vec<ModelConfig>, ConfigError> {
        let config = self.load_config().await?;
        Ok(config.model_configs)
    }

    /// 获取启用的模型配置
    pub async fn get_enabled_model_configs(&self) -> Result<Vec<ModelConfig>, ConfigError> {
        let config = self.load_config().await?;
        Ok(config.get_enabled_configs().into_iter().cloned().collect())
    }

    /// 更新模型缓存
    pub async fn update_model_cache(
        &self,
        provider: &ModelProvider,
        models: Vec<ModelInfo>,
        cache_hours: i64,
    ) -> Result<(), ConfigError> {
        let mut config = self.load_config().await?;

        let cache_key = match provider {
            ModelProvider::OpenAI => "openai".to_string(),
            ModelProvider::Anthropic => "anthropic".to_string(),
            ModelProvider::Google => "google".to_string(),
            ModelProvider::DeepSeek => "deepseek".to_string(),
            ModelProvider::AzureOpenAI => "azure_openai".to_string(),
            ModelProvider::Custom(name) => format!("custom_{}", name),
        };

        config.model_cache.insert(cache_key, models);
        config.update_cache_expiry(cache_hours);

        self.save_config(&config).await
    }

    /// 获取缓存的模型列表
    pub async fn get_cached_models(
        &self,
        provider: &ModelProvider,
    ) -> Result<Option<Vec<ModelInfo>>, ConfigError> {
        let config = self.load_config().await?;

        if config.is_cache_expired() {
            return Ok(None);
        }

        let cache_key = match provider {
            ModelProvider::OpenAI => "openai".to_string(),
            ModelProvider::Anthropic => "anthropic".to_string(),
            ModelProvider::Google => "google".to_string(),
            ModelProvider::DeepSeek => "deepseek".to_string(),
            ModelProvider::AzureOpenAI => "azure_openai".to_string(),
            ModelProvider::Custom(name) => format!("custom_{}", name),
        };

        Ok(config.model_cache.get(&cache_key).cloned())
    }

    /// 清除模型缓存
    pub async fn clear_model_cache(&self) -> Result<(), ConfigError> {
        let mut config = self.load_config().await?;
        config.model_cache.clear();
        config.cache_expires_at = None;
        self.save_config(&config).await
    }
}

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("存储错误: {0}")]
    StoreError(String),
    #[error("序列化错误: {0}")]
    SerializationError(String),
    #[error("配置迁移失败: {0}")]
    MigrationError(String),
    #[error("配置不存在")]
    ConfigNotFound,
}

impl From<tauri_plugin_store::Error> for ConfigError {
    fn from(err: tauri_plugin_store::Error) -> Self {
        ConfigError::StoreError(err.to_string())
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::SerializationError(err.to_string())
    }
}

/// 配置相关的 Tauri 命令
pub mod commands {
    use super::*;
    use tauri::State;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// 类型别名，简化状态管理
    pub type ConfigManagerState = Arc<Mutex<ConfigManager>>;

    /// 获取所有模型配置
    #[tauri::command]
    pub async fn get_model_configs(
        manager: State<'_, ConfigManagerState>,
    ) -> Result<Vec<ModelConfig>, String> {
        let manager = manager.lock().await;
        manager.get_all_model_configs()
            .await
            .map_err(|e| e.to_string())
    }

    /// 获取当前激活的模型配置
    #[tauri::command]
    pub async fn get_active_model_config(
        manager: State<'_, ConfigManagerState>,
    ) -> Result<Option<ModelConfig>, String> {
        let manager = manager.lock().await;
        manager.get_active_model_config()
            .await
            .map_err(|e| e.to_string())
    }

    /// 设置激活的模型配置
    #[tauri::command]
    pub async fn set_active_model_config(
        manager: State<'_, ConfigManagerState>,
        config_id: String,
    ) -> Result<bool, String> {
        let manager = manager.lock().await;
        manager.set_active_model_config(&config_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// 保存模型配置
    #[tauri::command]
    pub async fn save_model_config(
        manager: State<'_, ConfigManagerState>,
        config: ModelConfig,
    ) -> Result<(), String> {
        let manager = manager.lock().await;
        manager.add_model_config(config)
            .await
            .map_err(|e| e.to_string())
    }

    /// 更新模型配置
    #[tauri::command]
    pub async fn update_model_config(
        manager: State<'_, ConfigManagerState>,
        config_id: String,
        updates: ModelConfigUpdate,
    ) -> Result<bool, String> {
        let manager = manager.lock().await;
        manager.update_model_config(&config_id, updates)
            .await
            .map_err(|e| e.to_string())
    }

    /// 删除模型配置
    #[tauri::command]
    pub async fn delete_model_config(
        manager: State<'_, ConfigManagerState>,
        config_id: String,
    ) -> Result<bool, String> {
        let manager = manager.lock().await;
        manager.delete_model_config(&config_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// 清除模型缓存
    #[tauri::command]
    pub async fn clear_model_cache(
        manager: State<'_, ConfigManagerState>,
    ) -> Result<(), String> {
        let manager = manager.lock().await;
        manager.clear_model_cache()
            .await
            .map_err(|e| e.to_string())
    }
}