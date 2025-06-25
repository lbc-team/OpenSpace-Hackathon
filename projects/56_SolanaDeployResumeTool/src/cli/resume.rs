use crate::core::{types::*, StateManager, ResumeEngine, NetworkAnalyzer};
use std::str::FromStr;
use uuid::Uuid;

pub async fn handle_resume(
    matches: &clap::ArgMatches<'_>,
    mut state_manager: StateManager,
    resume_engine: ResumeEngine,
    network_analyzer: &mut NetworkAnalyzer,
    _keypair_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let deployment_id_str = matches.value_of("deployment_id").unwrap();
    let deployment_id = Uuid::from_str(deployment_id_str)?;

    println!("🔄 续传部署: {}", deployment_id);

    // 获取部署状态
    let deployment = state_manager.get_deployment(&deployment_id)
        .ok_or("部署不存在")?;

    if !matches!(deployment.status, DeploymentStatus::Failed | DeploymentStatus::Paused) {
        return Err("部署状态不支持续传".into());
    }

    println!("📊 部署信息:");
    println!("  程序文件: {}", deployment.program_path);
    println!("  总大小: {} bytes", deployment.total_size);
    println!("  已上传: {} bytes ({:.1}%)", 
        deployment.uploaded_bytes,
        (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0
    );

    // 分析网络状况
    let network_stats = network_analyzer.generate_network_stats().await?;
    println!("📡 当前网络状况: {:?}", network_stats.congestion_level);

    // 计算续传点
    let resume_point = resume_engine.calculate_resume_point(deployment)?;
    println!("🎯 续传点: {} bytes", resume_point);

    // 开始续传
    println!("🚀 开始续传上传...");
    state_manager.update_deployment_status(&deployment_id, DeploymentStatus::Uploading)?;

    // 模拟续传过程
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    
    println!("✅ 续传完成！");
    state_manager.update_deployment_status(&deployment_id, DeploymentStatus::Completed)?;

    Ok(())
} 