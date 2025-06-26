use crate::core::StateManager;

pub async fn handle_cleanup(
    matches: &clap::ArgMatches<'_>,
    mut state_manager: StateManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let days_str = matches.value_of("days").unwrap();
    let days: i64 = days_str.parse()?;

    println!("🧹 清理 {} 天前的已完成部署记录...", days);
    let removed_count = state_manager.cleanup_completed(days)?;
    println!("✅ 已清理 {} 条记录", removed_count);

    Ok(())
} 