use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use crate::core::types::*;

/// 性能优化器
pub struct PerformanceOptimizer {
    chunk_manager: ChunkManager,
    upload_scheduler: UploadScheduler,
    bandwidth_monitor: BandwidthMonitor,
    memory_manager: MemoryManager,
}

impl PerformanceOptimizer {
    pub fn new(config: &ResumeConfig) -> Self {
        Self {
            chunk_manager: ChunkManager::new(config.chunk_size),
            upload_scheduler: UploadScheduler::new(config.parallel_uploads),
            bandwidth_monitor: BandwidthMonitor::new(),
            memory_manager: MemoryManager::new(),
        }
    }

    /// 优化文件分块策略
    pub async fn optimize_chunking(&mut self, file_size: u64, network_stats: &NetworkStats) -> Result<ChunkStrategy> {
        // 根据网络状况调整分块大小
        let optimal_chunk_size = self.calculate_optimal_chunk_size(file_size, network_stats).await?;
        
        // 计算并行度
        let optimal_parallelism = self.calculate_optimal_parallelism(network_stats).await?;
        
        // 预分配内存
        self.memory_manager.reserve_memory(optimal_chunk_size * optimal_parallelism as u64)?;
        
        Ok(ChunkStrategy {
            chunk_size: optimal_chunk_size,
            parallelism: optimal_parallelism,
            total_chunks: (file_size + optimal_chunk_size - 1) / optimal_chunk_size,
            priority_order: self.calculate_chunk_priority(file_size, optimal_chunk_size),
        })
    }

    async fn calculate_optimal_chunk_size(&self, file_size: u64, network_stats: &NetworkStats) -> Result<u64> {
        let base_chunk_size = network_stats.optimal_chunk_size as u64;
        
        // 根据文件大小调整
        let size_factor = if file_size > 10 * 1024 * 1024 { // > 10MB
            2.0 // 大文件使用更大的分块
        } else if file_size < 1024 * 1024 { // < 1MB
            0.5 // 小文件使用更小的分块
        } else {
            1.0
        };
        
        // 根据网络拥堵调整
        let congestion_factor = match network_stats.congestion_level {
            CongestionLevel::Low => 1.5,
            CongestionLevel::Medium => 1.0,
            CongestionLevel::High => 0.7,
            CongestionLevel::Critical => 0.5,
        };
        
        let optimal_size = (base_chunk_size as f64 * size_factor * congestion_factor) as u64;
        
        // 确保在合理范围内
        Ok(optimal_size.max(1024).min(1024 * 1024)) // 1KB - 1MB
    }

    async fn calculate_optimal_parallelism(&self, network_stats: &NetworkStats) -> Result<usize> {
        let base_parallelism = match network_stats.congestion_level {
            CongestionLevel::Low => 8,
            CongestionLevel::Medium => 4,
            CongestionLevel::High => 2,
            CongestionLevel::Critical => 1,
        };
        
        // 根据带宽调整
        let bandwidth_factor = if network_stats.throughput_bps > 1024.0 * 1024.0 { // > 1MB/s
            1.5
        } else if network_stats.throughput_bps < 100.0 * 1024.0 { // < 100KB/s
            0.5
        } else {
            1.0
        };
        
        Ok(((base_parallelism as f64 * bandwidth_factor) as usize).max(1).min(16))
    }

    fn calculate_chunk_priority(&self, file_size: u64, chunk_size: u64) -> Vec<usize> {
        let total_chunks = (file_size + chunk_size - 1) / chunk_size;
        let mut priorities = Vec::new();
        
        // 优先上传文件开头和结尾的块，便于快速验证
        for i in 0..total_chunks.min(3) {
            priorities.push(i as usize);
        }
        
        // 然后是文件末尾的块
        for i in (total_chunks.saturating_sub(3)..total_chunks).rev() {
            if !priorities.contains(&(i as usize)) {
                priorities.push(i as usize);
            }
        }
        
        // 最后是中间的块
        for i in 3..total_chunks.saturating_sub(3) {
            priorities.push(i as usize);
        }
        
        priorities
    }
}

/// 分块管理器
pub struct ChunkManager {
    default_chunk_size: usize,
    chunk_cache: VecDeque<Vec<u8>>,
    max_cache_size: usize,
}

impl ChunkManager {
    pub fn new(default_chunk_size: usize) -> Self {
        Self {
            default_chunk_size,
            chunk_cache: VecDeque::new(),
            max_cache_size: 50, // 最多缓存50个分块
        }
    }

    /// 智能分块文件
    pub fn chunk_file(&mut self, data: &[u8], chunk_size: usize) -> Vec<Chunk> {
        let actual_chunk_size = if chunk_size == 0 {
            self.default_chunk_size
        } else {
            chunk_size
        };

        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = (offset + actual_chunk_size).min(data.len());
            let chunk_data = data[offset..end].to_vec();
            
            chunks.push(Chunk {
                id: chunks.len(),
                offset: offset as u64,
                size: chunk_data.len(),
                data: chunk_data,
                checksum: self.calculate_checksum(&data[offset..end]),
                retry_count: 0,
                last_attempt: None,
            });
            
            offset = end;
        }

        chunks
    }

    /// 计算校验和
    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// 验证分块完整性
    pub fn verify_chunk(&self, chunk: &Chunk) -> bool {
        let calculated_checksum = self.calculate_checksum(&chunk.data);
        calculated_checksum == chunk.checksum
    }
}

/// 上传调度器
pub struct UploadScheduler {
    semaphore: Arc<Semaphore>,
    active_uploads: Arc<RwLock<usize>>,
    max_parallel: usize,
}

impl UploadScheduler {
    pub fn new(max_parallel: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            active_uploads: Arc::new(RwLock::new(0)),
            max_parallel,
        }
    }

    /// 获取当前活跃上传数
    pub async fn get_active_count(&self) -> usize {
        *self.active_uploads.read().await
    }
}

/// 带宽监控器
pub struct BandwidthMonitor {
    samples: VecDeque<BandwidthSample>,
    max_samples: usize,
}

impl BandwidthMonitor {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            max_samples: 100,
        }
    }

    /// 获取当前带宽
    pub async fn get_current_bandwidth(&self) -> Result<f64> {
        if self.samples.is_empty() {
            return Ok(0.0);
        }

        // 计算最近10个样本的平均带宽
        let recent_samples = self.samples.iter().rev().take(10);
        let total_bandwidth: f64 = recent_samples.map(|s| s.bandwidth).sum();
        let count = self.samples.len().min(10);

        Ok(total_bandwidth / count as f64)
    }
}

/// 内存管理器
pub struct MemoryManager {
    allocated_memory: usize,
    max_memory: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            allocated_memory: 0,
            max_memory: 512 * 1024 * 1024, // 512MB最大内存使用
        }
    }

    /// 预留内存
    pub fn reserve_memory(&mut self, size: u64) -> Result<()> {
        let size = size as usize;
        if self.allocated_memory + size > self.max_memory {
            return Err(anyhow::anyhow!("内存不足，无法分配 {} 字节", size));
        }
        
        self.allocated_memory += size;
        Ok(())
    }

    /// 获取内存使用率
    pub fn get_usage_ratio(&self) -> f64 {
        self.allocated_memory as f64 / self.max_memory as f64
    }
}

// 数据结构定义
#[derive(Debug, Clone)]
pub struct ChunkStrategy {
    pub chunk_size: u64,
    pub parallelism: usize,
    pub total_chunks: u64,
    pub priority_order: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: usize,
    pub offset: u64,
    pub size: usize,
    pub data: Vec<u8>,
    pub checksum: String,
    pub retry_count: u32,
    pub last_attempt: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct BandwidthSample {
    pub timestamp: Instant,
    pub bytes_transferred: u64,
    pub duration: Duration,
    pub bandwidth: f64,
}
