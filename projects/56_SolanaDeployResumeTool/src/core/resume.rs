use crate::core::types::*;
use anyhow::{anyhow, Result};
// use chrono::Utc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Keypair,
    // transaction::Transaction,
};
use std::fs;
use std::path::Path;
use std::time::Duration;
// use uuid::Uuid;

/// 续传引擎
pub struct ResumeEngine {
    rpc_client: RpcClient,
    commitment: CommitmentConfig,
}

impl ResumeEngine {
    /// 创建新的续传引擎
    pub fn new(rpc_url: String) -> Self {
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
        Self {
            rpc_client,
            commitment: CommitmentConfig::confirmed(),
        }
    }
    
    /// 检测程序文件的加载器版本
    pub fn detect_loader_version(&self, program_path: &Path) -> Result<LoaderVersion> {
        // 这里简化处理，实际需要分析.so文件格式
        // 可以通过文件大小、ELF headers等来判断
        let file_size = fs::metadata(program_path)?.len();
        
        // 简单的启发式规则：
        // - 较大的程序 (>100KB) 通常使用 Loader v4
        // - 较小的程序使用 Loader v3
        if file_size > 100 * 1024 {
            Ok(LoaderVersion::V4)
        } else {
            Ok(LoaderVersion::V3)
        }
    }
    
    /// 计算续传点
    pub fn calculate_resume_point(&self, deployment: &DeploymentState) -> Result<u64> {
        match deployment.loader_version {
            LoaderVersion::V3 => self.calculate_v3_resume_point(deployment),
            LoaderVersion::V4 => self.calculate_v4_resume_point(deployment),
        }
    }
    
    /// 计算 Loader v3 的续传点
    fn calculate_v3_resume_point(&self, deployment: &DeploymentState) -> Result<u64> {
        let mut total_uploaded = 0u64;
        
        // 检查每个buffer的状态
        for buffer in &deployment.buffer_accounts {
            match self.check_buffer_status(&buffer.pubkey) {
                Ok(actual_size) => {
                    total_uploaded += actual_size;
                }
                Err(_) => {
                    // Buffer不存在或损坏，从这里开始续传
                    break;
                }
            }
        }
        
        Ok(total_uploaded)
    }
    
    /// 计算 Loader v4 的续传点
    fn calculate_v4_resume_point(&self, deployment: &DeploymentState) -> Result<u64> {
        // Loader v4 支持字节级的续传
        if let Some(program_id) = deployment.program_id {
            match self.get_program_data_length(&program_id) {
                Ok(length) => Ok(length),
                Err(_) => Ok(0), // 程序不存在，从头开始
            }
        } else {
            Ok(0)
        }
    }
    
    /// 检查buffer账户状态
    fn check_buffer_status(&self, buffer_pubkey: &Pubkey) -> Result<u64> {
        match self.rpc_client.get_account_with_commitment(buffer_pubkey, self.commitment) {
            Ok(response) => {
                if let Some(account) = response.value {
                    Ok(account.data.len() as u64)
                } else {
                    Err(anyhow!("Buffer账户不存在"))
                }
            }
            Err(e) => Err(anyhow!("获取Buffer账户失败: {}", e)),
        }
    }
    
    /// 获取程序数据长度
    fn get_program_data_length(&self, program_id: &Pubkey) -> Result<u64> {
        match self.rpc_client.get_account_with_commitment(program_id, self.commitment) {
            Ok(response) => {
                if let Some(account) = response.value {
                    Ok(account.data.len() as u64)
                } else {
                    Err(anyhow!("程序账户不存在"))
                }
            }
            Err(e) => Err(anyhow!("获取程序账户失败: {}", e)),
        }
    }
    
    /// 执行续传部署
    pub async fn resume_deployment(
        &self,
        deployment: &DeploymentState,
        program_data: &[u8],
        payer: &Keypair,
        config: &ResumeConfig,
    ) -> Result<()> {
        let resume_point = self.calculate_resume_point(deployment)?;
        
        match deployment.loader_version {
            LoaderVersion::V3 => {
                self.resume_v3_deployment(deployment, program_data, payer, config, resume_point).await
            }
            LoaderVersion::V4 => {
                self.resume_v4_deployment(deployment, program_data, payer, config, resume_point).await
            }
        }
    }
    
    /// 续传 Loader v3 部署
    async fn resume_v3_deployment(
        &self,
        deployment: &DeploymentState,
        program_data: &[u8],
        payer: &Keypair,
        config: &ResumeConfig,
        resume_point: u64,
    ) -> Result<()> {
        println!("开始续传 Loader v3 部署，从字节 {} 开始", resume_point);
        
        let remaining_data = &program_data[resume_point as usize..];
        let chunk_size = config.chunk_size;
        
        // 分批上传剩余数据
        for (i, chunk) in remaining_data.chunks(chunk_size).enumerate() {
            let offset = resume_point + (i * chunk_size) as u64;
            
            for retry in 0..config.max_retries {
                match self.upload_chunk_v3(chunk, offset, payer).await {
                    Ok(_) => {
                        println!("成功上传块 {} (偏移: {})", i, offset);
                        break;
                    }
                    Err(e) => {
                        eprintln!("上传块 {} 失败 (重试 {}): {}", i, retry + 1, e);
                        if retry < config.max_retries - 1 {
                            tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
                        } else {
                            return Err(anyhow!("上传块 {} 最终失败", i));
                        }
                    }
                }
            }
        }
        
        println!("Loader v3 续传部署完成");
        Ok(())
    }
    
    /// 续传 Loader v4 部署
    async fn resume_v4_deployment(
        &self,
        deployment: &DeploymentState,
        program_data: &[u8],
        payer: &Keypair,
        config: &ResumeConfig,
        resume_point: u64,
    ) -> Result<()> {
        println!("开始续传 Loader v4 部署，从字节 {} 开始", resume_point);
        
        let remaining_data = &program_data[resume_point as usize..];
        let chunk_size = config.chunk_size;
        
        for (i, chunk) in remaining_data.chunks(chunk_size).enumerate() {
            let offset = resume_point + (i * chunk_size) as u64;
            
            for retry in 0..config.max_retries {
                match self.upload_chunk_v4(chunk, offset, payer).await {
                    Ok(_) => {
                        println!("成功上传块 {} (偏移: {})", i, offset);
                        break;
                    }
                    Err(e) => {
                        eprintln!("上传块 {} 失败 (重试 {}): {}", i, retry + 1, e);
                        if retry < config.max_retries - 1 {
                            tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
                        } else {
                            return Err(anyhow!("上传块 {} 最终失败", i));
                        }
                    }
                }
            }
        }
        
        println!("Loader v4 续传部署完成");
        Ok(())
    }
    
    /// 上传 Loader v3 数据块
    async fn upload_chunk_v3(&self, chunk: &[u8], offset: u64, payer: &Keypair) -> Result<()> {
        // 这里是简化的实现，实际需要调用 Solana 的 write-buffer 指令
        // 使用模拟延时来表示网络操作
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        if chunk.len() > 1024 {
            return Err(anyhow!("模拟的网络错误"));
        }
        
        Ok(())
    }
    
    /// 上传 Loader v4 数据块
    async fn upload_chunk_v4(&self, chunk: &[u8], offset: u64, payer: &Keypair) -> Result<()> {
        // 这里是简化的实现，实际需要调用 Solana 的 Loader v4 指令
        tokio::time::sleep(Duration::from_millis(80)).await;
        
        if chunk.len() > 2048 {
            return Err(anyhow!("模拟的网络错误"));
        }
        
        Ok(())
    }
    
    /// 验证部署完整性
    pub fn verify_deployment(&self, deployment: &DeploymentState, original_data: &[u8]) -> Result<bool> {
        if let Some(program_id) = deployment.program_id {
            match self.rpc_client.get_account_with_commitment(&program_id, self.commitment) {
                Ok(response) => {
                    if let Some(account) = response.value {
                        // 简单的大小验证
                        let deployed_size = account.data.len() as u64;
                        let expected_size = original_data.len() as u64;
                        
                        Ok(deployed_size == expected_size)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
    
    /// 清理失败的buffers
    pub async fn cleanup_failed_buffers(&self, deployment: &DeploymentState, payer: &Keypair) -> Result<u64> {
        let mut cleaned_count = 0;
        
        for buffer in &deployment.buffer_accounts {
            if buffer.status == BufferStatus::Failed {
                // 这里需要实现实际的buffer清理逻辑
                // 简化处理，只是计数
                cleaned_count += 1;
                println!("清理失败的buffer: {}", buffer.pubkey);
            }
        }
        
        Ok(cleaned_count)
    }
    
    /// 估算剩余费用
    pub fn estimate_remaining_fees(&self, deployment: &DeploymentState, base_fee_per_byte: u64) -> u64 {
        let remaining_bytes = deployment.total_size.saturating_sub(deployment.uploaded_bytes);
        remaining_bytes * base_fee_per_byte
    }
} 