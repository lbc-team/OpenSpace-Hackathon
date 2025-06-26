use clap::{App, Arg, SubCommand};
use std::str::FromStr;
use tokio;
use uuid::Uuid;

mod core;
mod api;
mod cli;

use core::{
    types::*,
    StateManager, ResumeEngine, NetworkAnalyzer, FeeOptimizer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    tracing_subscriber::fmt::init();
    
    let matches = App::new("Solana Deploy Resume Tool")
        .version("0.1.0")
        .about("智能的Solana程序部署续传工具")
        .arg(
            Arg::with_name("rpc_url")
                .long("rpc-url")
                .value_name("URL")
                .help("Solana RPC节点URL")
                .default_value("https://api.devnet.solana.com")
                .global(true),
        )
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .short("k")
                .value_name("PATH")
                .help("密钥对文件路径")
                .default_value("~/.config/solana/id.json")
                .global(true),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("部署新程序")
                .arg(
                    Arg::with_name("program_file")
                        .long("program-file")
                        .value_name("PATH")
                        .help("程序.so文件路径")
                        .required(true),
                )
                .arg(
                    Arg::with_name("loader_version")
                        .long("loader-version")
                        .value_name("VERSION")
                        .help("加载器版本 (v3/v4)")
                        .possible_values(&["v3", "v4"])
                        .default_value("v4"),
                ),
        )
        .subcommand(
            SubCommand::with_name("resume")
                .about("续传失败的部署")
                .arg(
                    Arg::with_name("deployment_id")
                        .long("deployment-id")
                        .value_name("ID")
                        .help("部署ID")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("status")
                .about("查看部署状态")
                .arg(
                    Arg::with_name("deployment_id")
                        .long("deployment-id")
                        .value_name("ID")
                        .help("部署ID (可选，不指定则显示所有)")
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("列出所有部署")
                .arg(
                    Arg::with_name("resumable_only")
                        .long("resumable-only")
                        .help("只显示可续传的部署"),
                ),
        )
        .subcommand(
            SubCommand::with_name("cleanup")
                .about("清理已完成的部署记录")
                .arg(
                    Arg::with_name("days")
                        .long("days")
                        .value_name("DAYS")
                        .help("保留最近N天的记录")
                        .default_value("7"),
                ),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("启动Web服务器")
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .value_name("PORT")
                        .help("服务器端口")
                        .default_value("8080"),
                ),
        )
        .subcommand(
            SubCommand::with_name("analyze")
                .about("分析网络状况")
                .arg(
                    Arg::with_name("duration")
                        .long("duration")
                        .value_name("SECONDS")
                        .help("分析持续时间（秒）")
                        .default_value("60"),
                ),
        )
        .get_matches();

    // 获取全局参数
    let rpc_url = matches.value_of("rpc_url").unwrap().to_string();
    let keypair_path = matches.value_of("keypair").unwrap();

    // 初始化组件
    let state_manager = StateManager::new("./data/deployments.db")?;
    let resume_engine = ResumeEngine::new(rpc_url.clone());
    let mut network_analyzer = NetworkAnalyzer::new(rpc_url.clone());
    let mut fee_optimizer = FeeOptimizer::new(rpc_url.clone());

    match matches.subcommand() {
        ("deploy", Some(sub_matches)) => {
            cli::deploy::handle_deploy(
                sub_matches,
                state_manager,
                resume_engine,
                &mut network_analyzer,
                &mut fee_optimizer,
                keypair_path,
            )
            .await?;
        }
        ("resume", Some(sub_matches)) => {
            cli::resume::handle_resume(
                sub_matches,
                state_manager,
                resume_engine,
                &mut network_analyzer,
                keypair_path,
            )
            .await?;
        }
        ("status", Some(sub_matches)) => {
            cli::status::handle_status(sub_matches, &state_manager).await?;
        }
        ("list", Some(sub_matches)) => {
            cli::list::handle_list(sub_matches, &state_manager).await?;
        }
        ("cleanup", Some(sub_matches)) => {
            cli::cleanup::handle_cleanup(sub_matches, state_manager).await?;
        }
        ("server", Some(sub_matches)) => {
            cli::server::handle_server(sub_matches, rpc_url).await?;
        }
        ("analyze", Some(sub_matches)) => {
            cli::analyze::handle_analyze(sub_matches, &mut network_analyzer).await?;
        }
        _ => {
            println!("使用 --help 查看可用命令");
        }
    }

    Ok(())
} 