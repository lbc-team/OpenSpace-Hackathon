use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
// use std::collections::HashMap;
use uuid::Uuid;

/// 部署状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentState {
    pub id: Uuid,
    pub program_id: Option<Pubkey>,
    pub program_path: String,
    pub loader_version: LoaderVersion,
    pub total_size: u64,
    pub uploaded_bytes: u64,
    pub buffer_accounts: Vec<BufferInfo>,
    pub status: DeploymentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub failure_count: u32,
    pub last_error: Option<String>,
    pub network_stats: NetworkStats,
    pub cost_stats: CostStats,
}

/// 加载器版本
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoaderVersion {
    V3,
    V4,
}

/// 部署状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeploymentStatus {
    Initializing,
    Uploading,
    Paused,
    Failed,
    Completed,
    Cancelled,
}

/// Buffer信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferInfo {
    pub pubkey: Pubkey,
    pub size: u64,
    pub uploaded_size: u64,
    pub offset: u64,
    pub status: BufferStatus,
    pub created_at: DateTime<Utc>,
}

/// Buffer状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BufferStatus {
    Creating,
    Uploading,
    Completed,
    Failed,
}

/// 网络统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStats {
    pub latency_ms: f64,
    pub throughput_bps: f64,
    pub packet_loss_rate: f64,
    pub congestion_level: CongestionLevel,
    pub optimal_chunk_size: usize,
}

/// 网络拥堵等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for CongestionLevel {
    fn default() -> Self {
        CongestionLevel::Medium
    }
}

/// 成本统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostStats {
    pub total_fees_paid: u64,
    pub estimated_remaining_fees: u64,
    pub saved_fees: u64,
    pub transaction_count: u32,
    pub retry_count: u32,
}

/// 续传配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub chunk_size: usize,
    pub parallel_uploads: usize,
    pub auto_resume: bool,
    pub fee_optimization: bool,
}

impl Default for ResumeConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            chunk_size: 1024,
            parallel_uploads: 4,
            auto_resume: true,
            fee_optimization: true,
        }
    }
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub deployment_success_rate: f64,
    pub average_upload_time: f64,
    pub total_fees_saved: u64,
    pub network_efficiency: f64,
    pub buffer_reuse_rate: f64,
}

/// 部署事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEvent {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub event_type: EventType,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Started,
    Progress,
    Paused,
    Resumed,
    Failed,
    Completed,
    Error,
    Warning,
    Info,
}

/// 错误类型
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("网络错误: {0}")]
    Network(String),
    
    #[error("Solana RPC错误: {0}")]
    SolanaRpc(String),
    
    #[error("状态管理错误: {0}")]
    StateManagement(String),
    
    #[error("文件系统错误: {0}")]
    FileSystem(String),
    
    #[error("配置错误: {0}")]
    Configuration(String),
    
    #[error("部署被取消")]
    Cancelled,
    
    #[error("未知错误: {0}")]
    Unknown(String),
} 