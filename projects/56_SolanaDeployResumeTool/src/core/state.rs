use crate::core::types::*;
use anyhow::Result;
use chrono::Utc;
// use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// 状态管理器
pub struct StateManager {
    db: Db,
    deployments: HashMap<Uuid, DeploymentState>,
}

impl StateManager {
    /// 创建新的状态管理器
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db = sled::open(db_path)?;
        let mut deployments = HashMap::new();
        
        // 从数据库加载现有部署状态
        for item in db.iter() {
            let (key, value) = item?;
            let deployment_id = Uuid::from_slice(&key)?;
            let deployment_state: DeploymentState = serde_json::from_slice(&value)?;
            deployments.insert(deployment_id, deployment_state);
        }
        
        Ok(Self { db, deployments })
    }
    
    /// 创建新的部署状态
    pub fn create_deployment(&mut self, program_path: String, loader_version: LoaderVersion) -> Result<Uuid> {
        let deployment_id = Uuid::new_v4();
        let now = Utc::now();
        
        let deployment_state = DeploymentState {
            id: deployment_id,
            program_id: None,
            program_path,
            loader_version,
            total_size: 0,
            uploaded_bytes: 0,
            buffer_accounts: Vec::new(),
            status: DeploymentStatus::Initializing,
            created_at: now,
            updated_at: now,
            failure_count: 0,
            last_error: None,
            network_stats: NetworkStats::default(),
            cost_stats: CostStats::default(),
        };
        
        self.deployments.insert(deployment_id, deployment_state.clone());
        self.save_deployment(&deployment_state)?;
        
        Ok(deployment_id)
    }
    
    /// 获取部署状态
    pub fn get_deployment(&self, id: &Uuid) -> Option<&DeploymentState> {
        self.deployments.get(id)
    }
    
    /// 获取所有部署状态
    pub fn get_all_deployments(&self) -> Vec<&DeploymentState> {
        self.deployments.values().collect()
    }
    
    /// 更新部署状态
    pub fn update_deployment(&mut self, deployment: DeploymentState) -> Result<()> {
        let mut updated_deployment = deployment;
        updated_deployment.updated_at = Utc::now();
        
        self.deployments.insert(updated_deployment.id, updated_deployment.clone());
        self.save_deployment(&updated_deployment)?;
        
        Ok(())
    }
    
    /// 更新部署状态字段
    pub fn update_deployment_status(&mut self, id: &Uuid, status: DeploymentStatus) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.status = status;
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 更新上传进度
    pub fn update_upload_progress(&mut self, id: &Uuid, uploaded_bytes: u64) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.uploaded_bytes = uploaded_bytes;
            if uploaded_bytes >= deployment.total_size && deployment.total_size > 0 {
                deployment.status = DeploymentStatus::Completed;
            }
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 添加错误信息
    pub fn add_error(&mut self, id: &Uuid, error: String) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.last_error = Some(error);
            deployment.failure_count += 1;
            deployment.status = DeploymentStatus::Failed;
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 更新网络统计
    pub fn update_network_stats(&mut self, id: &Uuid, stats: NetworkStats) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.network_stats = stats;
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 更新成本统计
    pub fn update_cost_stats(&mut self, id: &Uuid, stats: CostStats) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.cost_stats = stats;
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 添加Buffer信息
    pub fn add_buffer(&mut self, id: &Uuid, buffer: BufferInfo) -> Result<()> {
        if let Some(mut deployment) = self.deployments.get(id).cloned() {
            deployment.buffer_accounts.push(buffer);
            self.update_deployment(deployment)?;
        }
        Ok(())
    }
    
    /// 查找可续传的部署
    pub fn find_resumable_deployments(&self) -> Vec<&DeploymentState> {
        self.deployments
            .values()
            .filter(|deployment| {
                matches!(deployment.status, DeploymentStatus::Failed | DeploymentStatus::Paused)
                    && deployment.uploaded_bytes > 0
                    && deployment.uploaded_bytes < deployment.total_size
            })
            .collect()
    }
    
    /// 清理已完成的部署（可选择保留多长时间）
    pub fn cleanup_completed(&mut self, days_to_keep: i64) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days_to_keep);
        let mut removed_count = 0;
        
        let to_remove: Vec<Uuid> = self
            .deployments
            .values()
            .filter(|deployment| {
                deployment.status == DeploymentStatus::Completed
                    && deployment.updated_at < cutoff_date
            })
            .map(|deployment| deployment.id)
            .collect();
        
        for id in to_remove {
            self.deployments.remove(&id);
            self.db.remove(id.as_bytes())?;
            removed_count += 1;
        }
        
        Ok(removed_count)
    }
    
    /// 获取性能指标
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        let total_deployments = self.deployments.len() as f64;
        if total_deployments == 0.0 {
            return PerformanceMetrics::default();
        }
        
        let successful_deployments = self
            .deployments
            .values()
            .filter(|d| d.status == DeploymentStatus::Completed)
            .count() as f64;
        
        let total_fees_saved = self
            .deployments
            .values()
            .map(|d| d.cost_stats.saved_fees)
            .sum();
        
        let average_upload_time = self
            .deployments
            .values()
            .filter(|d| d.status == DeploymentStatus::Completed)
            .map(|d| {
                (d.updated_at - d.created_at).num_seconds() as f64
            })
            .sum::<f64>() / successful_deployments.max(1.0);
        
        let buffer_reuse_count = self
            .deployments
            .values()
            .filter(|d| d.failure_count > 0 && d.status == DeploymentStatus::Completed)
            .count() as f64;
        
        PerformanceMetrics {
            deployment_success_rate: successful_deployments / total_deployments,
            average_upload_time,
            total_fees_saved,
            network_efficiency: 0.85, // 这里需要根据实际网络分析计算
            buffer_reuse_rate: buffer_reuse_count / total_deployments,
        }
    }
    
    /// 保存部署状态到数据库
    fn save_deployment(&self, deployment: &DeploymentState) -> Result<()> {
        let value = serde_json::to_vec(deployment)?;
        self.db.insert(deployment.id.as_bytes(), value)?;
        Ok(())
    }
    
    /// 删除部署状态
    pub fn delete_deployment(&mut self, id: &Uuid) -> Result<bool> {
        let existed = self.deployments.remove(id).is_some();
        if existed {
            self.db.remove(id.as_bytes())?;
        }
        Ok(existed)
    }
} 