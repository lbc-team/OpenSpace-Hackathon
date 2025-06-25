use crate::api;

pub async fn handle_server(
    matches: &clap::ArgMatches<'_>,
    rpc_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let port_str = matches.value_of("port").unwrap();
    let port: u16 = port_str.parse()?;

    println!("🌐 启动Web服务器...");
    println!("🔗 服务地址: http://localhost:{}", port);
    
    // 这里会调用API服务器模块
    api::server::start_server(port, rpc_url).await?;

    Ok(())
} 