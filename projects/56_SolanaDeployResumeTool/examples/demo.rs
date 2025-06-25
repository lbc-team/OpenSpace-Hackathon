use anyhow::Result;
use uuid::Uuid;

use solana_deploy_resume_tool::core::{
    StateManager, ResumeEngine, NetworkAnalyzer, FeeOptimizer,
    types::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Solana部署续传工具 Demo");
    println!("==========================");

    // 初始化组件
    let mut state_manager = StateManager::new("./demo_data")?;
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let _resume_engine = ResumeEngine::new(rpc_url.clone());
    let mut network_analyzer = NetworkAnalyzer::new(rpc_url.clone());
    let mut fee_optimizer = FeeOptimizer::new(rpc_url.clone());

    // Demo 1: 网络分析
    println!("\n📊 Demo 1: 网络状况分析");
    println!("-----------------------");
    
    let network_stats = network_analyzer.generate_network_stats().await?;
    println!("当前网络延迟: {:.1}ms", network_stats.latency_ms);
    println!("网络拥堵程度: {:?}", network_stats.congestion_level);
    println!("推荐最大块大小: {} bytes", network_stats.optimal_chunk_size);
    println!("网络吞吐量: {:.1} bytes/s", network_stats.throughput_bps);

    // Demo 2: 费用分析
    println!("\n💰 Demo 2: 费用优化分析");
    println!("-----------------------");
    
    let program_size = 1024 * 500; // 500KB示例程序
    
    let cost_stats = fee_optimizer.estimate_total_deployment_cost(
        program_size, 
        &LoaderVersion::V4, 
        &network_stats
    ).await?;
    println!("预估部署费用: {} lamports", cost_stats.estimated_remaining_fees);
    println!("交易数量: {}", cost_stats.transaction_count);
    println!("平均每交易费用: {} lamports", 
        cost_stats.estimated_remaining_fees / cost_stats.transaction_count as u64);

    // Demo 3: 创建模拟部署
    println!("\n🎯 Demo 3: 创建模拟部署");
    println!("-----------------------");
    
    let deployment_id = state_manager.create_deployment(
        "./examples/demo_program.so".to_string(), 
        LoaderVersion::V4
    )?;
    println!("创建部署ID: {}", deployment_id);
    
    // 更新部署信息
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        let mut updated_deployment = deployment.clone();
        updated_deployment.total_size = program_size;
        updated_deployment.network_stats = network_stats.clone();
        updated_deployment.cost_stats = cost_stats.clone();
        state_manager.update_deployment(updated_deployment)?;
    }
    
    // 查看部署状态
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        println!("部署状态: {:?}", deployment.status);
        println!("程序大小: {} bytes", deployment.total_size);
        println!("已上传字节: {}", deployment.uploaded_bytes);
        let progress = if deployment.total_size > 0 {
            (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0
        } else {
            0.0
        };
        println!("进度: {:.1}%", progress);
    }

    // Demo 4: 模拟续传场景
    println!("\n🔄 Demo 4: 模拟续传功能");
    println!("-----------------------");
    
    // 模拟部分上传完成的情况
    state_manager.update_upload_progress(&deployment_id, program_size / 2)?;
    state_manager.update_deployment_status(&deployment_id, DeploymentStatus::Failed)?;
    
    if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
        println!("模拟部署失败，已上传: {} / {} bytes", 
            deployment.uploaded_bytes, deployment.total_size);
        
        // 计算续传节省
        let savings = fee_optimizer.calculate_resume_savings(deployment, &network_stats).await?;
        println!("续传可节省费用: {} lamports", savings);
        
        // 成本效益分析
        let analysis = fee_optimizer.cost_benefit_analysis(deployment, &network_stats).await?;
        println!("建议操作: {}", analysis.recommended_action);
        println!("盈亏平衡点: {} lamports", analysis.break_even_point);
    }

    // Demo 5: 查找可续传的部署
    println!("\n🔍 Demo 5: 查找可续传部署");
    println!("-----------------------");
    
    let resumable = state_manager.find_resumable_deployments();
    println!("找到 {} 个可续传的部署", resumable.len());
    
    for deployment in resumable {
        println!("  - ID: {}, 进度: {:.1}%, 状态: {:?}", 
            deployment.id,
            (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0,
            deployment.status
        );
    }

    // Demo 6: 性能指标
    println!("\n📈 Demo 6: 性能指标统计");
    println!("-----------------------");
    
    let metrics = state_manager.get_performance_metrics();
    println!("部署成功率: {:.1}%", metrics.deployment_success_rate * 100.0);
    println!("总节省费用: {} lamports", metrics.total_fees_saved);
    println!("平均上传时间: {:.1} 秒", metrics.average_upload_time);
    println!("Buffer复用率: {:.1}%", metrics.buffer_reuse_rate * 100.0);
    println!("网络效率: {:.1}%", metrics.network_efficiency * 100.0);

    // Demo 7: 网络监控
    println!("\n🌐 Demo 7: 网络监控 (30秒)");
    println!("-----------------------");
    println!("开始监控网络状况...");
    
    let stats_history = network_analyzer.monitor_network_changes(30).await?;
    println!("监控完成，收集了 {} 个数据点", stats_history.len());
    
    if let Some(prediction) = network_analyzer.predict_best_deployment_time(&stats_history) {
        println!("最佳部署时间预测: {}", prediction);
    }

    // Demo 8: 清理演示
    println!("\n🧹 Demo 8: 清理演示数据");
    println!("-----------------------");
    
    let cleaned = state_manager.cleanup_completed(7)?; // 保留7天内的记录
    println!("清理了 {} 个过期的部署记录", cleaned);

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
    
    Ok(())
}
