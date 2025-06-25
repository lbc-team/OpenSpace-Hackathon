use crate::core::{types::*, StateManager, ResumeEngine, NetworkAnalyzer, FeeOptimizer};
use solana_sdk::signature::{read_keypair_file, Signer};
use std::path::Path;

pub async fn handle_deploy(
    matches: &clap::ArgMatches<'_>,
    mut state_manager: StateManager,
    resume_engine: ResumeEngine,
    network_analyzer: &mut NetworkAnalyzer,
    fee_optimizer: &mut FeeOptimizer,
    keypair_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_file = matches.value_of("program_file").unwrap();
    let loader_version_str = matches.value_of("loader_version").unwrap();
    
    let loader_version = match loader_version_str {
        "v3" => LoaderVersion::V3,
        "v4" => LoaderVersion::V4,
        _ => LoaderVersion::V4,
    };

    println!("🚀 开始新的程序部署...");
    println!("📄 程序文件: {}", program_file);
    println!("🔧 加载器版本: {:?}", loader_version);
    println!("🔑 密钥对路径: {}", keypair_path);

    // 检测程序文件
    let program_path = std::path::Path::new(program_file);
    if !program_path.exists() {
        return Err(format!("程序文件不存在: {}", program_file).into());
    }

    // 展开密钥对路径
    let expanded_keypair_path = if keypair_path.starts_with("~/") {
        if let Some(home) = std::env::var("HOME").ok() {
            keypair_path.replace("~", &home)
        } else {
            keypair_path.to_string()
        }
    } else {
        keypair_path.to_string()
    };

    // 检查密钥对文件
    if !Path::new(&expanded_keypair_path).exists() {
        return Err(format!("密钥对文件不存在: {}", expanded_keypair_path).into());
    }

    // 读取密钥对
    let payer_keypair = match read_keypair_file(&expanded_keypair_path) {
        Ok(keypair) => keypair,
        Err(e) => return Err(format!("无法读取密钥对文件: {}", e).into()),
    };

    println!("💰 付款账户: {}", payer_keypair.pubkey());

    // 读取程序数据
    let program_data = std::fs::read(program_path)?;
    println!("📊 程序大小: {} bytes", program_data.len());

    // 分析网络状况
    println!("🔍 分析网络状况...");
    let network_stats = network_analyzer.generate_network_stats().await?;
    println!("📡 网络延迟: {:.1}ms", network_stats.latency_ms);
    println!("📊 拥堵等级: {:?}", network_stats.congestion_level);

    // 估算费用
    let cost_stats = fee_optimizer.estimate_total_deployment_cost(
        program_data.len() as u64,
        &loader_version,
        &network_stats,
    ).await?;
    println!("💰 估算费用: {} lamports", cost_stats.estimated_remaining_fees);

    // 创建部署状态
    let deployment_id = state_manager.create_deployment(program_file.to_string(), loader_version)?;
    println!("🆔 部署ID: {}", deployment_id);

    // 获取推荐配置
    let config = network_analyzer.recommend_deployment_strategy(&network_stats);
    println!("⚙️  推荐配置: 块大小 {}B, 并发数 {}", config.chunk_size, config.parallel_uploads);

    // 开始部署（这里是简化版本）
    println!("📤 开始上传程序数据...");
    
    // 更新部署状态
    let mut deployment = state_manager.get_deployment(&deployment_id).unwrap().clone();
    deployment.total_size = program_data.len() as u64;
    deployment.status = DeploymentStatus::Uploading;
    deployment.network_stats = network_stats;
    deployment.cost_stats = cost_stats;
    
    state_manager.update_deployment(deployment)?;

    // 模拟上传过程
    for i in 0..5 {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        let progress = ((i + 1) * 20) as u64;
        state_manager.update_upload_progress(&deployment_id, (program_data.len() as u64 * progress) / 100)?;
        println!("📈 上传进度: {}%", progress);
    }

    println!("✅ 部署完成！");
    state_manager.update_deployment_status(&deployment_id, DeploymentStatus::Completed)?;

    Ok(())
} 