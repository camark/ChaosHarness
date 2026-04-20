//! ACP Service -整合 ACP 协议到 RustHarness
//!
//! 提供：
//! - ACP 服务器运行器
//! - 与远程 ACP 代理的桥梁
//! - 本地工具到 ACP 技能的映射

#![allow(dead_code)]

use crate::acp;
use crate::config::Settings;
use anyhow::Result;
use tracing::info;

/// ACP 服务配置
#[derive(Debug, Clone)]
pub struct AcpServiceConfig {
    /// 是否启用 ACP 服务器
    pub enable_server: bool,
    /// ACP 服务器端口
    pub server_port: u16,
    /// 远程 ACP 代理 URL 列表
    pub remote_agents: Vec<String>,
    /// API 密钥（用于认证）
    pub api_key: Option<String>,
}

impl Default for AcpServiceConfig {
    fn default() -> Self {
        Self {
            enable_server: false,
            server_port: 8080,
            remote_agents: Vec::new(),
            api_key: None,
        }
    }
}

/// ACP 服务管理器
pub struct AcpService {
    config: AcpServiceConfig,
    server_state: Option<acp::server::AcpServerState>,
}

impl AcpService {
    /// 创建新的 ACP 服务
    pub fn new(config: AcpServiceConfig) -> Self {
        Self {
            config,
            server_state: None,
        }
    }

    /// 从 Settings 创建配置
    pub fn from_settings(_settings: &Settings) -> AcpServiceConfig {
        // 从 settings.json 或环境变量读取 ACP 配置
        // TODO: 在 Settings 中添加 acp 字段
        AcpServiceConfig::default()
    }

    /// 启动 ACP 服务器
    pub async fn start_server(&mut self, port: u16) -> Result<()> {
        info!("Starting ACP server on port {}", port);

        let base_url = format!("http://localhost:{}", port);
        self.server_state = Some(acp::server::AcpServerState::new(&base_url));

        // 在实际使用中，这里会返回一个 handle 用于停止服务器
        // 目前我们直接运行服务器直到 shutdown
        acp::server::run_acp_server(port).await?;

        Ok(())
    }

    /// 停止 ACP 服务器
    pub fn stop_server(&mut self) {
        info!("Stopping ACP server");
        self.server_state = None;
    }

    /// 获取服务器状态
    pub fn server_state(&self) -> Option<&acp::server::AcpServerState> {
        self.server_state.as_ref()
    }

    /// 创建到远程 ACP 代理的客户端
    pub fn create_client(&self, base_url: &str) -> acp::client::AcpClient {
        if let Some(ref api_key) = self.config.api_key {
            acp::client::AcpClient::with_auth(base_url, api_key)
        } else {
            acp::client::AcpClient::new(base_url)
        }
    }

    /// 发现并连接远程 ACP 代理
    pub async fn connect_to_agent(&self, base_url: &str) -> Result<acp::types::AgentCard> {
        let mut client = self.create_client(base_url);
        let agent_card = client.discover().await?;
        Ok(agent_card.clone())
    }

    /// 获取配置
    pub fn config(&self) -> &AcpServiceConfig {
        &self.config
    }
}

/// 运行 ACP 服务器（独立模式）
pub async fn run_acp_service(port: u16) -> Result<()> {
    let mut service = AcpService::new(AcpServiceConfig::default());
    service.start_server(port).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acp_service_config_default() {
        let config = AcpServiceConfig::default();
        assert!(!config.enable_server);
        assert_eq!(config.server_port, 8080);
        assert!(config.remote_agents.is_empty());
    }

    #[test]
    fn test_acp_service_creation() {
        let config = AcpServiceConfig {
            enable_server: true,
            server_port: 9000,
            remote_agents: vec!["http://localhost:8080".to_string()],
            api_key: Some("test-key".to_string()),
        };

        let service = AcpService::new(config);
        assert!(service.config().enable_server);
        assert_eq!(service.config().server_port, 9000);
    }
}
