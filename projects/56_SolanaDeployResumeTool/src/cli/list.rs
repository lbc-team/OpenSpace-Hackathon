use crate::core::StateManager;

pub async fn handle_list(
    matches: &clap::ArgMatches<'_>,
    state_manager: &StateManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let resumable_only = matches.is_present("resumable_only");
    
    let deployments = if resumable_only {
        state_manager.find_resumable_deployments()
    } else {
        state_manager.get_all_deployments()
    };

    if deployments.is_empty() {
        if resumable_only {
            println!("📭 没有找到可续传的部署");
        } else {
            println!("📭 没有找到任何部署记录");
        }
    } else {
        println!("📋 部署列表:");
        for deployment in deployments {
            println!("🆔 {}", deployment.id);
            println!("📄 {}", deployment.program_path);
            println!("📊 状态: {:?}", deployment.status);
            println!("📈 进度: {}/{} bytes ({:.1}%)",
                deployment.uploaded_bytes,
                deployment.total_size,
                if deployment.total_size > 0 {
                    (deployment.uploaded_bytes as f64 / deployment.total_size as f64) * 100.0
                } else { 0.0 }
            );
            println!("---");
        }
    }

    Ok(())
} 