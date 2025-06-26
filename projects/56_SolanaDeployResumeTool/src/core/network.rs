use crate::core::types::*;
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// 网络分析器
pub struct NetworkAnalyzer {
    rpc_url: String,
    recent_measurements: Vec<LatencyMeasurement>,
}

#[derive(Debug, Clone)]
struct LatencyMeasurement {
    timestamp: Instant,
    latency_ms: f64,
    success: bool,
}

impl NetworkAnalyzer {
    /// 创建新的网络分析器
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            recent_measurements: Vec::new(),
        }
    }
    
    /// 测量网络延迟
    pub async fn measure_latency(&mut self) -> Result<f64> {
        let start = Instant::now();
        
        // 发送简单的健康检查请求
        let client = reqwest::Client::new();
        let request_future = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getHealth"
            }))
            .send();
        
        match timeout(Duration::from_secs(5), request_future).await {
            Ok(Ok(response)) => {
                let latency_ms = start.elapsed().as_millis() as f64;
                let success = response.status().is_success();
                
                self.recent_measurements.push(LatencyMeasurement {
                    timestamp: start,
                    latency_ms,
                    success,
                });
                
                // 只保留最近100次测量
                if self.recent_measurements.len() > 100 {
                    self.recent_measurements.remove(0);
                }
                
                Ok(latency_ms)
            }
            Ok(Err(e)) => {
                self.recent_measurements.push(LatencyMeasurement {
                    timestamp: start,
                    latency_ms: 5000.0, // 超时视为5秒延迟
                    success: false,
                });
                
                Err(anyhow::anyhow!("网络请求失败: {}", e))
            }
            Err(_) => {
                self.recent_measurements.push(LatencyMeasurement {
                    timestamp: start,
                    latency_ms: 5000.0,
                    success: false,
                });
                
                Err(anyhow::anyhow!("网络请求超时"))
            }
        }
    }
    
    /// 计算平均延迟
    pub fn get_average_latency(&self) -> f64 {
        if self.recent_measurements.is_empty() {
            return 100.0; // 默认值
        }
        
        let total: f64 = self.recent_measurements
            .iter()
            .map(|m| m.latency_ms)
            .sum();
        
        total / self.recent_measurements.len() as f64
    }
    
    /// 计算成功率
    pub fn get_success_rate(&self) -> f64 {
        if self.recent_measurements.is_empty() {
            return 0.95; // 默认值
        }
        
        let success_count = self.recent_measurements
            .iter()
            .filter(|m| m.success)
            .count();
        
        success_count as f64 / self.recent_measurements.len() as f64
    }
    
    /// 估算网络吞吐量
    pub async fn estimate_throughput(&mut self) -> Result<f64> {
        // 发送一个较大的测试请求来估算吞吐量
        let test_data = vec![0u8; 1024]; // 1KB测试数据
        let start = Instant::now();
        
        let client = reqwest::Client::new();
        
        match client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getAccountInfo",
                "params": [
                    "11111111111111111111111111111112", // System Program
                    {"encoding": "base64"}
                ]
            }))
            .send()
            .await
        {
            Ok(_response) => {
                let elapsed = start.elapsed().as_secs_f64();
                let bytes_per_second = 1024.0 / elapsed;
                Ok(bytes_per_second)
            }
            Err(e) => Err(anyhow::anyhow!("吞吐量测试失败: {}", e)),
        }
    }
    
    /// 检测网络拥堵等级
    pub async fn detect_congestion_level(&mut self) -> Result<CongestionLevel> {
        let _ = self.measure_latency().await;
        let avg_latency = self.get_average_latency();
        let success_rate = self.get_success_rate();
        
        // 根据延迟和成功率判断拥堵等级
        match (avg_latency, success_rate) {
            (lat, rate) if lat < 100.0 && rate > 0.95 => Ok(CongestionLevel::Low),
            (lat, rate) if lat < 300.0 && rate > 0.90 => Ok(CongestionLevel::Medium),
            (lat, rate) if lat < 1000.0 && rate > 0.80 => Ok(CongestionLevel::High),
            _ => Ok(CongestionLevel::Critical),
        }
    }
    
    /// 计算最优块大小
    pub fn calculate_optimal_chunk_size(&self, congestion_level: CongestionLevel) -> usize {
        match congestion_level {
            CongestionLevel::Low => 8192,    // 8KB
            CongestionLevel::Medium => 4096, // 4KB
            CongestionLevel::High => 2048,   // 2KB
            CongestionLevel::Critical => 1024, // 1KB
        }
    }
    
    /// 生成网络统计信息
    pub async fn generate_network_stats(&mut self) -> Result<NetworkStats> {
        let latency = match self.measure_latency().await {
            Ok(lat) => lat,
            Err(_) => self.get_average_latency(),
        };
        
        let throughput = match self.estimate_throughput().await {
            Ok(tput) => tput,
            Err(_) => 1024.0 * 8.0, // 默认8KB/s
        };
        
        let packet_loss_rate = 1.0 - self.get_success_rate();
        let congestion_level = self.detect_congestion_level().await?;
        let optimal_chunk_size = self.calculate_optimal_chunk_size(congestion_level.clone());
        
        Ok(NetworkStats {
            latency_ms: latency,
            throughput_bps: throughput,
            packet_loss_rate,
            congestion_level,
            optimal_chunk_size,
        })
    }
    
    /// 推荐部署策略
    pub fn recommend_deployment_strategy(&self, network_stats: &NetworkStats) -> ResumeConfig {
        let mut config = ResumeConfig::default();
        
        match network_stats.congestion_level {
            CongestionLevel::Low => {
                config.chunk_size = 8192;
                config.parallel_uploads = 8;
                config.retry_delay_ms = 500;
                config.max_retries = 2;
            }
            CongestionLevel::Medium => {
                config.chunk_size = 4096;
                config.parallel_uploads = 4;
                config.retry_delay_ms = 1000;
                config.max_retries = 3;
            }
            CongestionLevel::High => {
                config.chunk_size = 2048;
                config.parallel_uploads = 2;
                config.retry_delay_ms = 2000;
                config.max_retries = 5;
            }
            CongestionLevel::Critical => {
                config.chunk_size = 1024;
                config.parallel_uploads = 1;
                config.retry_delay_ms = 5000;
                config.max_retries = 10;
            }
        }
        
        config
    }
    
    /// 监控网络状况变化
    pub async fn monitor_network_changes(&mut self, duration_secs: u64) -> Result<Vec<NetworkStats>> {
        let mut stats_history = Vec::new();
        let interval = Duration::from_secs(10); // 每10秒测量一次
        let end_time = Instant::now() + Duration::from_secs(duration_secs);
        
        while Instant::now() < end_time {
            match self.generate_network_stats().await {
                Ok(stats) => {
                    stats_history.push(stats);
                    println!(
                        "网络状况: 延迟 {:.1}ms, 拥堵等级 {:?}",
                        stats_history.last().unwrap().latency_ms,
                        stats_history.last().unwrap().congestion_level
                    );
                }
                Err(e) => {
                    eprintln!("网络监控失败: {}", e);
                }
            }
            
            tokio::time::sleep(interval).await;
        }
        
        Ok(stats_history)
    }
    
    /// 预测最佳部署时间
    pub fn predict_best_deployment_time(&self, stats_history: &[NetworkStats]) -> Option<String> {
        if stats_history.is_empty() {
            return None;
        }
        
        // 找到网络状况最好的时间段
        let best_stats = stats_history
            .iter()
            .min_by(|a, b| {
                let score_a = a.latency_ms + (a.packet_loss_rate * 1000.0);
                let score_b = b.latency_ms + (b.packet_loss_rate * 1000.0);
                score_a.partial_cmp(&score_b).unwrap()
            })?;
        
        match best_stats.congestion_level {
            CongestionLevel::Low => Some("网络状况良好，建议立即部署".to_string()),
            CongestionLevel::Medium => Some("网络状况一般，可以部署但建议降低并发".to_string()),
            CongestionLevel::High => Some("网络拥堵，建议等待或使用保守策略".to_string()),
            CongestionLevel::Critical => Some("网络严重拥堵，强烈建议延后部署".to_string()),
        }
    }
} 