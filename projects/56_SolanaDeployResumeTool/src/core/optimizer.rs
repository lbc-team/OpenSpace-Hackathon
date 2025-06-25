use crate::core::types::*;
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    // pubkey::Pubkey,
};
use std::collections::HashMap;

/// 费用优化器
pub struct FeeOptimizer {
    rpc_client: RpcClient,
    fee_history: Vec<FeeRecord>,
    buffer_registry: HashMap<String, Vec<BufferInfo>>,
}

#[derive(Debug, Clone)]
struct FeeRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    base_fee: u64,
    priority_fee: u64,
    congestion_multiplier: f64,
}

impl FeeOptimizer {
    /// 创建新的费用优化器
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
            fee_history: Vec::new(),
            buffer_registry: HashMap::new(),
        }
    }
    
    /// 获取当前基础费用
    pub async fn get_current_base_fee(&mut self) -> Result<u64> {
        // 模拟获取当前网络基础费用
        // 实际应该通过 RPC 调用获取
        match self.rpc_client.get_recent_blockhash() {
            Ok(_) => {
                let base_fee = 5000; // 5000 lamports 作为基础费用
                
                self.fee_history.push(FeeRecord {
                    timestamp: chrono::Utc::now(),
                    base_fee,
                    priority_fee: 0,
                    congestion_multiplier: 1.0,
                });
                
                Ok(base_fee)
            }
            Err(e) => Err(anyhow::anyhow!("获取基础费用失败: {}", e)),
        }
    }
    
    /// 计算优先费用
    pub async fn calculate_priority_fee(&mut self, congestion_level: &CongestionLevel) -> Result<u64> {
        let base_priority = match congestion_level {
            CongestionLevel::Low => 0,
            CongestionLevel::Medium => 1000,    // 1000 lamports
            CongestionLevel::High => 5000,      // 5000 lamports
            CongestionLevel::Critical => 10000, // 10000 lamports
        };
        
        // 根据历史数据调整优先费用
        let adjusted_priority = if self.fee_history.len() > 10 {
            let recent_avg = self.fee_history
                .iter()
                .rev()
                .take(10)
                .map(|record| record.priority_fee)
                .sum::<u64>() / 10;
            
            std::cmp::max(base_priority, recent_avg)
        } else {
            base_priority
        };
        
        Ok(adjusted_priority)
    }
    
    /// 估算总部署费用
    pub async fn estimate_total_deployment_cost(
        &mut self,
        program_size: u64,
        loader_version: &LoaderVersion,
        network_stats: &NetworkStats,
    ) -> Result<CostStats> {
        let base_fee = self.get_current_base_fee().await?;
        let priority_fee = self.calculate_priority_fee(&network_stats.congestion_level).await?;
        
        // 根据加载器版本计算不同的费用结构
        let (transaction_count, bytes_per_transaction) = match loader_version {
            LoaderVersion::V3 => {
                // Loader v3 需要更多交易来处理buffer
                let bytes_per_tx = network_stats.optimal_chunk_size as u64;
                let tx_count = (program_size + bytes_per_tx - 1) / bytes_per_tx;
                (tx_count, bytes_per_tx)
            }
            LoaderVersion::V4 => {
                // Loader v4 更高效
                let bytes_per_tx = (network_stats.optimal_chunk_size * 2) as u64;
                let tx_count = (program_size + bytes_per_tx - 1) / bytes_per_tx;
                (tx_count, bytes_per_tx)
            }
        };
        
        let total_base_fees = transaction_count * base_fee;
        let total_priority_fees = transaction_count * priority_fee;
        let total_fees = total_base_fees + total_priority_fees;
        
        // 根据网络拥堵调整费用
        let congestion_multiplier = match network_stats.congestion_level {
            CongestionLevel::Low => 1.0,
            CongestionLevel::Medium => 1.2,
            CongestionLevel::High => 1.5,
            CongestionLevel::Critical => 2.0,
        };
        
        let adjusted_total = (total_fees as f64 * congestion_multiplier) as u64;
        
        Ok(CostStats {
            total_fees_paid: 0,
            estimated_remaining_fees: adjusted_total,
            saved_fees: 0,
            transaction_count: transaction_count as u32,
            retry_count: 0,
        })
    }
    
    /// 计算续传可节省的费用
    pub async fn calculate_resume_savings(
        &mut self,
        deployment: &DeploymentState,
        network_stats: &NetworkStats,
    ) -> Result<u64> {
        let total_size = deployment.total_size;
        let uploaded_size = deployment.uploaded_bytes;
        let remaining_size = total_size.saturating_sub(uploaded_size);
        
        if remaining_size == 0 {
            return Ok(0);
        }
        
        // 计算如果重新开始需要的费用
        let full_cost = self.estimate_total_deployment_cost(
            total_size,
            &deployment.loader_version,
            network_stats,
        ).await?;
        
        // 计算续传所需的费用
        let resume_cost = self.estimate_total_deployment_cost(
            remaining_size,
            &deployment.loader_version,
            network_stats,
        ).await?;
        
        let savings = full_cost.estimated_remaining_fees.saturating_sub(resume_cost.estimated_remaining_fees);
        Ok(savings)
    }
    
    /// Buffer复用策略分析
    pub fn analyze_buffer_reuse_opportunities(&mut self, program_hash: String) -> Vec<BufferInfo> {
        self.buffer_registry
            .get(&program_hash)
            .cloned()
            .unwrap_or_default()
    }
    
    /// 注册可复用的Buffer
    pub fn register_reusable_buffer(&mut self, program_hash: String, buffer: BufferInfo) {
        self.buffer_registry
            .entry(program_hash)
            .or_insert_with(Vec::new)
            .push(buffer);
    }
    
    /// 清理过期的Buffer记录
    pub fn cleanup_expired_buffers(&mut self, hours_to_keep: i64) -> usize {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(hours_to_keep);
        let mut removed_count = 0;
        
        for buffers in self.buffer_registry.values_mut() {
            let initial_len = buffers.len();
            buffers.retain(|buffer| buffer.created_at > cutoff_time);
            removed_count += initial_len - buffers.len();
        }
        
        removed_count
    }
    
    /// 成本效益分析
    pub async fn cost_benefit_analysis(
        &mut self,
        deployment: &DeploymentState,
        network_stats: &NetworkStats,
    ) -> Result<CostBenefitReport> {
        let potential_savings = self.calculate_resume_savings(deployment, network_stats).await?;
        let resume_cost = self.estimate_total_deployment_cost(
            deployment.total_size.saturating_sub(deployment.uploaded_bytes),
            &deployment.loader_version,
            network_stats,
        ).await?;
        
        let time_cost_factor = match network_stats.congestion_level {
            CongestionLevel::Low => 1.0,
            CongestionLevel::Medium => 1.5,
            CongestionLevel::High => 2.0,
            CongestionLevel::Critical => 3.0,
        };
        
        let recommended_action = if potential_savings > resume_cost.estimated_remaining_fees / 2 {
            "建议立即续传".to_string()
        } else if network_stats.congestion_level == CongestionLevel::Critical {
            "建议等待网络状况改善".to_string()
        } else {
            "建议评估时间成本后决定".to_string()
        };
        
        Ok(CostBenefitReport {
            potential_savings,
            resume_cost: resume_cost.estimated_remaining_fees,
            time_cost_factor,
            recommended_action,
            break_even_point: potential_savings / 2,
        })
    }
    
    /// 获取费用优化建议
    pub fn get_optimization_recommendations(&self, deployment: &DeploymentState) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // 基于失败次数的建议
        if deployment.failure_count > 2 {
            recommendations.push(OptimizationRecommendation {
                category: "重试策略".to_string(),
                description: "考虑增加重试延迟或减少并发数".to_string(),
                priority: RecommendationPriority::High,
                estimated_savings: 10000, // 估算节省的lamports
            });
        }
        
        // 基于网络状况的建议
        match deployment.network_stats.congestion_level {
            CongestionLevel::Critical => {
                recommendations.push(OptimizationRecommendation {
                    category: "时机选择".to_string(),
                    description: "网络严重拥堵，建议延后部署".to_string(),
                    priority: RecommendationPriority::High,
                    estimated_savings: 50000,
                });
            }
            CongestionLevel::High => {
                recommendations.push(OptimizationRecommendation {
                    category: "策略调整".to_string(),
                    description: "减少块大小和并发数".to_string(),
                    priority: RecommendationPriority::Medium,
                    estimated_savings: 20000,
                });
            }
            _ => {}
        }
        
        // 基于加载器版本的建议
        if deployment.loader_version == LoaderVersion::V3 && deployment.total_size > 200 * 1024 {
            recommendations.push(OptimizationRecommendation {
                category: "技术升级".to_string(),
                description: "大程序建议使用Loader v4".to_string(),
                priority: RecommendationPriority::Medium,
                estimated_savings: 30000,
            });
        }
        
        recommendations
    }
    
    /// 更新费用记录
    pub fn update_fee_record(&mut self, cost_stats: &CostStats) {
        if let Some(last_record) = self.fee_history.last_mut() {
            last_record.priority_fee = cost_stats.total_fees_paid / cost_stats.transaction_count as u64;
        }
    }
}

/// 成本效益报告
#[derive(Debug, Clone)]
pub struct CostBenefitReport {
    pub potential_savings: u64,
    pub resume_cost: u64,
    pub time_cost_factor: f64,
    pub recommended_action: String,
    pub break_even_point: u64,
}

/// 优化建议
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub category: String,
    pub description: String,
    pub priority: RecommendationPriority,
    pub estimated_savings: u64,
}

/// 建议优先级
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
} 