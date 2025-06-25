use crate::core::NetworkAnalyzer;

pub async fn handle_analyze(
    matches: &clap::ArgMatches<'_>,
    network_analyzer: &mut NetworkAnalyzer,
) -> Result<(), Box<dyn std::error::Error>> {
    let duration_str = matches.value_of("duration").unwrap();
    let duration: u64 = duration_str.parse()?;

    println!("🔍 开始网络分析，持续 {} 秒...", duration);
    
    let stats_history = network_analyzer.monitor_network_changes(duration).await?;
    
    println!("\n📊 分析结果:");
    if let Some(recommendation) = network_analyzer.predict_best_deployment_time(&stats_history) {
        println!("💡 建议: {}", recommendation);
    }

    let avg_latency: f64 = stats_history.iter().map(|s| s.latency_ms).sum::<f64>() / stats_history.len() as f64;
    println!("📡 平均延迟: {:.1}ms", avg_latency);

    Ok(())
} 