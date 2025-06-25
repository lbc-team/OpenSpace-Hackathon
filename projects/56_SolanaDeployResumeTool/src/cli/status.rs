use crate::core::{types::*, StateManager};
use std::str::FromStr;
use uuid::Uuid;

pub async fn handle_status(
    matches: &clap::ArgMatches<'_>,
    state_manager: &StateManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(deployment_id_str) = matches.value_of("deployment_id") {
        let deployment_id = Uuid::from_str(deployment_id_str)?;
        
        if let Some(deployment) = state_manager.get_deployment(&deployment_id) {
            print_deployment_status(deployment);
        } else {
            println!("❌ 部署不存在: {}", deployment_id);
        }
    } else {
        let deployments = state_manager.get_all_deployments();
        if deployments.is_empty() {
            println!("📭 没有找到任何部署记录");
        } else {
            println!("📋 所有部署状态:");
            for deployment in deployments {
                print_deployment_status(deployment);
                println!("---");
            }
        }
    }

    Ok(())
}

fn print_deployment_status(deployment: &DeploymentState) {
    println!("🆔 部署ID: {}", deployment.id);
    println!("📄 程序: {}", deployment.program_path);
    println!("📊 状态: {:?}", deployment.status);
    println!("🔧 加载器: {:?}", deployment.loader_version);
    println!("📈 进度: {}/{} bytes ({:.1}%)",
        deployment.uploaded_bytes,
        deployment.total_size,
        if deployment.total_size > 0 {
            (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0
        } else { 0.0 }
    );
    println!("⏰ 创建时间: {}", deployment.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("🔄 更新时间: {}", deployment.updated_at.format("%Y-%m-%d %H:%M:%S"));
    if deployment.failure_count > 0 {
        println!("❌ 失败次数: {}", deployment.failure_count);
        if let Some(ref error) = deployment.last_error {
            println!("💬 最后错误: {}", error);
        }
    }
    println!("💰 费用统计: 已付 {} lamports, 预估剩余 {} lamports",
        deployment.cost_stats.total_fees_paid,
        deployment.cost_stats.estimated_remaining_fees
    );
} 