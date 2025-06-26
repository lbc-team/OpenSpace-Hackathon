use anyhow::Result;

use solana_deploy_resume_tool::core::{
    StateManager,
    types::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Solana部署续传工具离线Demo");
    println!("=============================");
    println!("这个演示使用模拟数据，不需要网络连接");

    // 初始化状态管理器
    let mut state_manager = StateManager::new("./demo_offline_data")?;

    // Demo 1: 创建模拟部署
    println!("\n🎯 Demo 1: 创建模拟部署");
    println!("-----------------------");
    
    let deployment_id = state_manager.create_deployment(
        "./examples/demo_program.so".to_string(), 
        LoaderVersion::V4
    )?;
    println!("创建部署ID: {}", deployment_id);
    
    // 模拟网络和费用统计数据
    let network_stats = NetworkStats {
        latency_ms: 150.0,
        throughput_bps: 1024.0 * 50.0, // 50KB/s
        packet_loss_rate: 0.05,
        congestion_level: CongestionLevel::Medium,
        optimal_chunk_size: 4096,
    };
    
    let cost_stats = CostStats {
        total_fees_paid: 0,
        estimated_remaining_fees: 250000, // 0.25 SOL
        saved_fees: 0,
        transaction_count: 50,
        retry_count: 0,
    };
    
    // 更新部署信息
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        let mut updated_deployment = deployment.clone();
        updated_deployment.total_size = 1024 * 500; // 500KB程序
        updated_deployment.network_stats = network_stats.clone();
        updated_deployment.cost_stats = cost_stats.clone();
        state_manager.update_deployment(updated_deployment)?;
    }
    
    // 查看部署状态
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        println!("部署状态: {:?}", deployment.status);
        println!("程序大小: {} bytes", deployment.total_size);
        println!("预估费用: {} lamports", deployment.cost_stats.estimated_remaining_fees);
        println!("交易数量: {}", deployment.cost_stats.transaction_count);
    }

    // Demo 2: 模拟网络状况分析
    println!("\n📊 Demo 2: 网络状况分析");
    println!("-----------------------");
    
    println!("当前网络延迟: {:.1}ms", network_stats.latency_ms);
    println!("网络拥堵程度: {:?}", network_stats.congestion_level);
    println!("推荐最大块大小: {} bytes", network_stats.optimal_chunk_size);
    println!("网络吞吐量: {:.1} bytes/s", network_stats.throughput_bps);
    println!("数据包丢失率: {:.1}%", network_stats.packet_loss_rate * 100.0);

    // Demo 3: 模拟部分上传和失败
    println!("\n🔄 Demo 3: 模拟续传场景");
    println!("-----------------------");
    
    // 模拟上传了一半
    state_manager.update_upload_progress(&deployment_id, 256 * 1024)?; // 256KB
    state_manager.update_deployment_status(&deployment_id, DeploymentStatus::Failed)?;
    state_manager.add_error(&deployment_id, "网络连接中断".to_string())?;
    
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        println!("模拟部署失败，已上传: {} / {} bytes", 
            deployment.uploaded_bytes, deployment.total_size);
        println!("失败次数: {}", deployment.failure_count);
        println!("最后错误: {:?}", deployment.last_error);
        
        let progress = (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0;
        println!("完成进度: {:.1}%", progress);
        
        // 计算剩余费用
        let remaining_size = deployment.total_size - deployment.uploaded_bytes;
        let remaining_tx = (remaining_size + 8192 - 1) / 8192; // 假设8KB每个交易
        let remaining_fees = remaining_tx * 5000; // 假设每交易5000 lamports
        
        println!("剩余交易数: {}", remaining_tx);
        println!("预估剩余费用: {} lamports", remaining_fees);
        
        // 计算节省的费用
        let total_original_fees = deployment.cost_stats.estimated_remaining_fees;
        let savings = total_original_fees.saturating_sub(remaining_fees);
        println!("续传可节省: {} lamports ({:.1}%)", 
            savings, 
            (savings as f64 / total_original_fees as f64) * 100.0
        );
    }

    // Demo 4: 创建更多测试部署
    println!("\n🎲 Demo 4: 创建多个测试部署");
    println!("-----------------------");
    
    // 创建一个成功的部署
    let success_id = state_manager.create_deployment(
        "./examples/success_program.so".to_string(), 
        LoaderVersion::V3
    )?;
    
    if let Some(deployment) = state_manager.get_deployment(&success_id) {
        let mut updated = deployment.clone();
        updated.total_size = 1024 * 200; // 200KB
        updated.uploaded_bytes = updated.total_size; // 全部上传完成
        updated.status = DeploymentStatus::Completed;
        updated.cost_stats = CostStats {
            total_fees_paid: 150000,
            estimated_remaining_fees: 0,
            saved_fees: 50000,
            transaction_count: 25,
            retry_count: 2,
        };
        state_manager.update_deployment(updated)?;
    }
    
    // 创建一个暂停的部署
    let paused_id = state_manager.create_deployment(
        "./examples/paused_program.so".to_string(), 
        LoaderVersion::V4
    )?;
    
    if let Some(deployment) = state_manager.get_deployment(&paused_id) {
        let mut updated = deployment.clone();
        updated.total_size = 1024 * 800; // 800KB
        updated.uploaded_bytes = 1024 * 300; // 300KB已上传
        updated.status = DeploymentStatus::Paused;
        state_manager.update_deployment(updated)?;
    }
    
    println!("创建了3个测试部署:");
    println!("  - 失败的部署: {} (进度: 50%)", deployment_id);
    println!("  - 成功的部署: {} (进度: 100%)", success_id);
    println!("  - 暂停的部署: {} (进度: 37.5%)", paused_id);

    // Demo 5: 查找可续传的部署
    println!("\n🔍 Demo 5: 查找可续传部署");
    println!("-----------------------");
    
    let resumable = state_manager.find_resumable_deployments();
    println!("找到 {} 个可续传的部署", resumable.len());
    
    for deployment in resumable {
        let progress = if deployment.total_size > 0 {
            (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0
        } else {
            0.0
        };
        println!("  - ID: {}", deployment.id);
        println!("    状态: {:?}", deployment.status);
        println!("    进度: {:.1}%", progress);
        println!("    失败次数: {}", deployment.failure_count);
        if let Some(error) = &deployment.last_error {
            println!("    最后错误: {}", error);
        }
        println!();
    }

    // Demo 6: 所有部署概览
    println!("📋 Demo 6: 所有部署概览");
    println!("-----------------------");
    
    let all_deployments = state_manager.get_all_deployments();
    println!("总部署数: {}", all_deployments.len());
    
    let mut by_status = std::collections::HashMap::new();
    for deployment in &all_deployments {
        *by_status.entry(&deployment.status).or_insert(0) += 1;
    }
    
    for (status, count) in by_status {
        println!("  {:?}: {} 个", status, count);
    }

    // Demo 7: 性能指标
    println!("\n📈 Demo 7: 性能指标统计");
    println!("-----------------------");
    
    let metrics = state_manager.get_performance_metrics();
    println!("部署成功率: {:.1}%", metrics.deployment_success_rate * 100.0);
    println!("总节省费用: {} lamports", metrics.total_fees_saved);
    println!("平均上传时间: {:.1} 秒", metrics.average_upload_time);
    println!("Buffer复用率: {:.1}%", metrics.buffer_reuse_rate * 100.0);
    println!("网络效率: {:.1}%", metrics.network_efficiency * 100.0);

    // Demo 8: 推荐的续传配置
    println!("\n⚙️ Demo 8: 推荐的续传配置");
    println!("-----------------------");
    
    let config = match network_stats.congestion_level {
        CongestionLevel::Low => ResumeConfig {
            chunk_size: 8192,
            parallel_uploads: 8,
            retry_delay_ms: 500,
            max_retries: 2,
            auto_resume: true,
            fee_optimization: true,
        },
        CongestionLevel::Medium => ResumeConfig {
            chunk_size: 4096,
            parallel_uploads: 4,
            retry_delay_ms: 1000,
            max_retries: 3,
            auto_resume: true,
            fee_optimization: true,
        },
        CongestionLevel::High => ResumeConfig {
            chunk_size: 2048,
            parallel_uploads: 2,
            retry_delay_ms: 2000,
            max_retries: 5,
            auto_resume: true,
            fee_optimization: true,
        },
        CongestionLevel::Critical => ResumeConfig {
            chunk_size: 1024,
            parallel_uploads: 1,
            retry_delay_ms: 5000,
            max_retries: 10,
            auto_resume: false,
            fee_optimization: true,
        },
    };
    
    println!("基于当前网络状况 ({:?}) 的推荐配置:", network_stats.congestion_level);
    println!("  块大小: {} bytes", config.chunk_size);
    println!("  并发上传数: {}", config.parallel_uploads);
    println!("  重试延迟: {} ms", config.retry_delay_ms);
    println!("  最大重试次数: {}", config.max_retries);
    println!("  自动续传: {}", if config.auto_resume { "启用" } else { "禁用" });
    println!("  费用优化: {}", if config.fee_optimization { "启用" } else { "禁用" });

    // Demo 9: 清理演示
    println!("\n🧹 Demo 9: 清理演示数据");
    println!("-----------------------");
    
    let cleaned = state_manager.cleanup_completed(0)?; // 清理所有已完成的记录
    println!("清理了 {} 个已完成的部署记录", cleaned);

    println!("\n✅ Demo完成！");
    println!("\n🎯 接下来你可以尝试：");
    println!("  1. 使用CLI工具部署真实程序:");
    println!("     cargo run -- deploy --program-path <your-program.so>");
    println!("  2. 查看所有部署状态:");
    println!("     cargo run -- list");
    println!("  3. 分析网络状况:");
    println!("     cargo run -- analyze");
    println!("  4. 启动Web界面:");
    println!("     cargo run -- server --port 8080");
    println!("  5. 续传失败的部署:");
    println!("     cargo run -- resume --deployment-id <id>");
    println!("  6. 运行在线版本demo (需要网络连接):");
    println!("     cargo run --example demo");
    
    Ok(())
} 