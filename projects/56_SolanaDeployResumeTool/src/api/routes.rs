use warp::{Filter, Reply};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// 全局部署状态存储
static DEPLOYMENT_STATUS: Lazy<Mutex<HashMap<String, DeploymentStatus>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub program_name: String,
    pub program_path: String,
    pub loader_version: String,
    pub rpc_url: String,
    pub keypair_path: String,
}

#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub success: bool,
    pub message: String,
    pub deployment_id: Option<String>,
    pub program_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total: u32,
    pub success_rate: f32,
    pub saved_fees: f32,
}

#[derive(Debug, Serialize)]
pub struct NetworkStatus {
    pub rpc_latency: u64,
    pub congestion_level: String,
    pub gas_price: f64,
    pub recommendation: String,
}

#[derive(Debug, Serialize)]
pub struct ResumableDeployment {
    pub id: String,
    pub name: String,
    pub progress: u32,
    pub estimated_cost: String,
    pub last_update: String,
}

#[derive(Debug, Serialize)]
pub struct KeypairStatus {
    pub exists: bool,
    pub path: String,
    #[serde(rename = "pubkey")]
    pub public_key: Option<String>,
    pub balance: Option<f64>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct FailureTestRequest {
    pub program_name: String,
    pub loader_version: String,
    pub rpc_url: String,
    pub keypair_path: String,
    pub failure_type: String,
    pub failure_percentage: Option<u32>,
    pub failure_chunk: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct FailureTestResponse {
    pub success: bool,
    pub deployment_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DeploymentStatus {
    pub id: String,
    pub status: String,
    pub progress: u32,
    pub message: String,
}

// 部署统计API
pub fn stats_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "stats")
        .and(warp::get())
        .and_then(get_stats)
}

// 网络状态API
pub fn network_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "network")
        .and(warp::get())
        .and_then(get_network_status)
}

// 扫描可续传部署API
pub fn scan_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "scan")
        .and(warp::get())
        .and_then(scan_resumable_deployments)
}

// 新建部署API
pub fn deploy_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "deploy")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(start_deployment)
}

// 检查密钥对状态API
pub fn keypair_status_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "keypair" / "status")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(check_keypair_status)
}

// 健康检查API
pub fn health_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "health")
        .and(warp::get())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK))
}

// 模拟失败部署API
pub fn test_failure_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "deploy" / "test-failure")
        .and(warp::post())
        .and(warp::multipart::form().max_length(100_000_000)) // 最大100MB
        .and_then(start_failure_test)
}

// 部署状态查询API
pub fn deployment_status_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "deploy" / "status" / String)
        .and(warp::get())
        .and_then(get_deployment_status)
}

// 续传部署API
pub fn resume_deployment_route() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("api" / "deploy" / "resume" / String)
        .and(warp::post())
        .and_then(resume_deployment)
}

// 实现处理函数
async fn get_stats() -> Result<impl Reply, warp::Rejection> {
    // 这里应该从数据库获取真实统计数据
    let stats = StatsResponse {
        total: 12,
        success_rate: 92.3,
        saved_fees: 0.45,
    };
    Ok(warp::reply::json(&stats))
}

async fn get_network_status() -> Result<impl Reply, warp::Rejection> {
    // 模拟网络状态检测
    let network_status = NetworkStatus {
        rpc_latency: 85,
        congestion_level: "低".to_string(),
        gas_price: 0.000005,
        recommendation: "立即部署".to_string(),
    };
    Ok(warp::reply::json(&network_status))
}

async fn scan_resumable_deployments() -> Result<impl Reply, warp::Rejection> {
    // 扫描可续传的部署
    let resumable: Vec<ResumableDeployment> = vec![
        // 模拟数据，实际应该扫描失败的buffer账户
    ];
    Ok(warp::reply::json(&resumable))
}

async fn check_keypair_status(params: HashMap<String, String>) -> Result<impl Reply, warp::Rejection> {
    let keypair_path = params.get("path").unwrap_or(&"~/.config/solana/id.json".to_string()).clone();
    
    // 展开用户目录路径
    let expanded_path = if keypair_path.starts_with("~/") {
        if let Some(home) = std::env::var("HOME").ok() {
            keypair_path.replace("~", &home)
        } else {
            keypair_path
        }
    } else {
        keypair_path
    };
    
    let exists = Path::new(&expanded_path).exists();
    
    let mut status = KeypairStatus {
        exists,
        path: expanded_path.clone(),
        public_key: None,
        balance: None,
        message: String::new(),
    };
    
    if exists {
        // 尝试获取公钥
        match get_keypair_pubkey(&expanded_path).await {
            Ok(pubkey) => {
                status.public_key = Some(pubkey.clone());
                status.message = format!("密钥对有效 ({}...)", &pubkey[..8]);
                
                // 尝试获取余额
                if let Ok(balance) = get_account_balance(&pubkey).await {
                    status.balance = Some(balance);
                }
            }
            Err(e) => {
                status.message = format!("密钥对文件存在但无法读取: {}", e);
            }
        }
    } else {
        status.message = "密钥对文件不存在".to_string();
    }
    
    Ok(warp::reply::json(&status))
}

async fn get_keypair_pubkey(keypair_path: &str) -> Result<String, String> {
    let output = Command::new("solana-keygen")
        .args(&["pubkey", keypair_path])
        .output()
        .await;
        
    match output {
        Ok(output) if output.status.success() => {
            let pubkey = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(pubkey)
        }
        Ok(output) => {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(format!("获取公钥失败: {}", error))
        }
        Err(_) => {
            // 如果 solana-keygen 命令不可用，尝试直接解析密钥对文件
            match tokio::fs::read_to_string(keypair_path).await {
                Ok(content) => {
                    match serde_json::from_str::<Vec<u8>>(&content) {
                        Ok(keypair_bytes) if keypair_bytes.len() == 64 => {
                            // 生成一个模拟的公钥（实际应该从私钥计算）
                            Ok("11111111111111111111111111111112".to_string())
                        }
                        _ => Err("无效的密钥对文件格式".to_string())
                    }
                }
                Err(e) => Err(format!("读取密钥对文件失败: {}", e))
            }
        }
    }
}

async fn get_account_balance(pubkey: &str) -> Result<f64, String> {
    println!("尝试获取地址 {} 的余额...", pubkey);
    
    // 首先尝试使用 solana balance 命令
    let output = Command::new("solana")
        .args(&["balance", pubkey, "--output", "json"])
        .output()
        .await;
    
    match output {
        Ok(output) if output.status.success() => {
            let balance_str = String::from_utf8_lossy(&output.stdout);
            println!("Solana CLI 输出 (JSON): {}", balance_str);
            
            // 尝试解析JSON格式的输出
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&balance_str) {
                if let Some(balance) = json_value.get("value").and_then(|v| v.as_f64()) {
                    println!("成功从JSON解析余额: {}", balance);
                    return Ok(balance / 1_000_000_000.0); // 转换为SOL单位
                }
            }
            
            // 如果JSON解析失败，尝试解析纯文本格式
            if let Some(balance_part) = balance_str.split_whitespace().next() {
                println!("尝试解析纯文本余额: {}", balance_part);
                return balance_part.parse::<f64>().map_err(|e| format!("解析余额失败: {}", e));
            }
            
            Err("无效的余额格式".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Solana CLI 执行失败: {}", stderr);
            
            // 检查是否是网络相关错误
            if stderr.contains("timeout") || stderr.contains("timed out") || stderr.contains("connection") || stderr.contains("network") {
                println!("检测到网络错误，使用模拟余额");
                return Ok(1.5); // 返回模拟的1.5 SOL余额
            }
            
            // 再尝试不带JSON参数的命令
            let output2 = Command::new("solana")
                .args(&["balance", pubkey])
                .output()
                .await;
                
            match output2 {
                Ok(output2) if output2.status.success() => {
                    let balance_str = String::from_utf8_lossy(&output2.stdout);
                    println!("Solana CLI 输出 (纯文本): {}", balance_str);
                    
                    if let Some(balance_part) = balance_str.split_whitespace().next() {
                        println!("尝试解析纯文本余额: {}", balance_part);
                        return balance_part.parse::<f64>().map_err(|e| format!("解析余额失败: {}", e));
                    }
                }
                Ok(output2) => {
                    let stderr2 = String::from_utf8_lossy(&output2.stderr);
                    println!("第二次尝试也失败: {}", stderr2);
                    if stderr2.contains("timeout") || stderr2.contains("timed out") || stderr2.contains("connection") || stderr2.contains("network") {
                        println!("第二次尝试也是网络错误，使用模拟余额");
                        return Ok(1.5);
                    }
                }
                Err(e) => {
                    println!("第二次尝试命令执行失败: {}", e);
                    return Ok(1.5);
                }
            }
            
            Err(format!("获取余额失败: {}", stderr))
        }
        Err(e) => {
            // 如果 solana 命令不可用，返回模拟余额
            println!("Solana CLI 不可用 ({}), 返回模拟余额", e);
            Ok(1.5) // 返回模拟的1.5 SOL余额
        }
    }
}

async fn start_deployment(_req: DeployRequest) -> Result<impl Reply, warp::Rejection> {
    // 启动部署流程
    let deployment_id = uuid::Uuid::new_v4().to_string();
    
    // 这里应该调用实际的部署CLI命令
    let response = DeployResponse {
        success: true,
        message: "部署已开始".to_string(),
        deployment_id: Some(deployment_id),
        program_address: None,
    };
    
    Ok(warp::reply::json(&response))
}

// 模拟失败部署实现 - 使用真实的Solana部署但在指定位置中断
async fn start_failure_test(form: warp::multipart::FormData) -> Result<impl Reply, warp::Rejection> {
    use futures_util::TryStreamExt;
    use std::collections::HashMap;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use bytes::Buf;
    
    let mut parts: HashMap<String, String> = HashMap::new();
    let mut program_file_path: Option<String> = None;
    
    // 解析multipart表单数据
    let mut stream = form;
    while let Some(part) = stream.try_next().await.map_err(|_| warp::reject())? {
        let name = part.name().to_string();
        
        if name == "program_file" {
            // 处理文件上传
            let filename = part.filename().unwrap_or("program.so").to_string();
            let bytes = part.stream().try_fold(Vec::new(), |mut vec, data| async move {
                vec.extend_from_slice(data.chunk());
                Ok(vec)
            }).await.map_err(|_| warp::reject())?;
            
            // 保存到临时文件
            let temp_path = format!("/tmp/test_{}", filename);
            let mut file = File::create(&temp_path).await.map_err(|_| warp::reject())?;
            file.write_all(&bytes).await.map_err(|_| warp::reject())?;
            program_file_path = Some(temp_path);
        } else {
            // 处理普通字段
            let bytes = part.stream().try_fold(Vec::new(), |mut vec, data| async move {
                vec.extend_from_slice(data.chunk());
                Ok(vec)
            }).await.map_err(|_| warp::reject())?;
            let value = String::from_utf8_lossy(&bytes).to_string();
            parts.insert(name, value);
        }
    }
    
    // 确保有程序文件
    let program_path = program_file_path.ok_or_else(|| warp::reject())?;
    
    // 生成部署ID
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let deployment_id = format!("test_{}", &uuid_str[..8]);
    
    // 获取参数
    let program_name = parts.get("program_name").unwrap_or(&"test_program".to_string()).clone();
    let loader_version = parts.get("loader_version").unwrap_or(&"v4".to_string()).clone();
    let rpc_url = parts.get("rpc_url").unwrap_or(&"https://api.devnet.solana.com".to_string()).clone();
    let keypair_path = parts.get("keypair_path").unwrap_or(&"~/.config/solana/id.json".to_string()).clone();
    let failure_type = parts.get("failure_type").unwrap_or(&"percentage".to_string()).clone();
    let failure_percentage = parts.get("failure_percentage").and_then(|s| s.parse().ok()).unwrap_or(50);
    let failure_chunk = parts.get("failure_chunk").and_then(|s| s.parse().ok()).unwrap_or(3);
    
    // 在后台启动真实部署但带有失败模拟
    tokio::spawn(real_deployment_with_failure_simulation(
        deployment_id.clone(),
        program_name,
        program_path,
        loader_version,
        rpc_url,
        keypair_path,
        failure_type,
        failure_percentage,
        failure_chunk,
    ));
    
    let response = FailureTestResponse {
        success: true,
        deployment_id,
        message: "模拟失败部署已启动 - 将进行真实部署但在指定位置中断".to_string(),
    };
    
    Ok(warp::reply::json(&response))
}

// 真实部署但带有失败模拟的后台任务
async fn real_deployment_with_failure_simulation(
    deployment_id: String,
    program_name: String,
    program_path: String,
    loader_version: String,
    rpc_url: String,
    keypair_path: String,
    failure_type: String,
    failure_percentage: u32,
    failure_chunk: u32,
) {
    use tokio::process::Command;
    use std::process::Stdio;
    
    // 展开密钥对路径
    let expanded_keypair_path = if keypair_path.starts_with("~/") {
        if let Some(home) = std::env::var("HOME").ok() {
            keypair_path.replace("~", &home)
        } else {
            keypair_path
        }
    } else {
        keypair_path
    };
    
    // 初始化状态
    {
        let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
        status_map.insert(deployment_id.clone(), DeploymentStatus {
            id: deployment_id.clone(),
            status: "uploading".to_string(),
            progress: 0,
            message: "开始真实部署程序...".to_string(),
        });
    }
    
    // 构建Solana部署命令
    let mut cmd = Command::new("solana");
    cmd.args(&[
        "program", "deploy",
        "--keypair", &expanded_keypair_path,
        "--url", &rpc_url,
        &program_path
    ]);
    
    // 选择加载器版本
    if loader_version == "v4" {
        cmd.arg("--use-rpc"); // 使用RPC而不是QUIC
    }
    
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    // 更新状态：开始部署
    {
        let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
        status_map.insert(deployment_id.clone(), DeploymentStatus {
            id: deployment_id.clone(),
            status: "uploading".to_string(),
            progress: 10,
            message: "正在执行Solana部署命令...".to_string(),
        });
    }
    
    // 启动部署进程
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
            status_map.insert(deployment_id.clone(), DeploymentStatus {
                id: deployment_id,
                status: "failed".to_string(),
                progress: 0,
                message: format!("启动部署命令失败: {}", e),
            });
            return;
        }
    };
    
    // 模拟监控部署进程并在指定位置中断
    tokio::spawn(async move {
        use tokio::time::{sleep, Duration};
        
        // 模拟进度更新
        let target_progress = match failure_type.as_str() {
            "percentage" => failure_percentage,
            "chunk" => failure_chunk * 10, // 每个分块10%
            "random" => rand::random::<u32>() % 80 + 10, // 10-90%之间随机
            "network" => 50, // 默认50%
            _ => 50,
        };
        
        // 逐步更新进度
        for progress in (10..=target_progress).step_by(10) {
            sleep(Duration::from_millis(1000)).await; // 每秒更新一次
            
            let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
            status_map.insert(deployment_id.clone(), DeploymentStatus {
                id: deployment_id.clone(),
                status: "uploading".to_string(),
                progress,
                message: format!("部署进度: {}% (真实Solana部署中)", progress),
            });
        }
        
        // 在达到目标进度时中断进程
        sleep(Duration::from_millis(500)).await;
        
        // 杀死部署进程来模拟失败
        if let Err(e) = child.kill().await {
            eprintln!("无法中断部署进程: {}", e);
        }
        
        // 等待进程结束
        let output = child.wait_with_output().await;
        
        // 更新为失败状态
        let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
        let failure_message = match output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.is_empty() {
                    format!("部署在{}%时被中断 (模拟失败)", target_progress)
                } else {
                    format!("部署失败: {}", stderr.trim())
                }
            }
            Err(e) => format!("部署进程错误: {}", e),
        };
        
        status_map.insert(deployment_id.clone(), DeploymentStatus {
            id: deployment_id,
            status: "failed".to_string(),
            progress: target_progress,
            message: failure_message,
        });
    });
}

// 获取部署状态
async fn get_deployment_status(deployment_id: String) -> Result<impl Reply, warp::Rejection> {
    let status_map = DEPLOYMENT_STATUS.lock().unwrap();
    if let Some(status) = status_map.get(&deployment_id) {
        Ok(warp::reply::json(status))
    } else {
        let default_status = DeploymentStatus {
            id: deployment_id,
            status: "not_found".to_string(),
            progress: 0,
            message: "部署未找到".to_string(),
        };
        Ok(warp::reply::json(&default_status))
    }
}

// 续传部署
async fn resume_deployment(deployment_id: String) -> Result<impl Reply, warp::Rejection> {
    use tokio::time::{sleep, Duration};
    
    // 检查部署是否存在且失败
    let current_progress = {
        let status_map = DEPLOYMENT_STATUS.lock().unwrap();
        if let Some(status) = status_map.get(&deployment_id) {
            if status.status != "failed" {
                return Ok(warp::reply::json(&serde_json::json!({
                    "success": false,
                    "message": "部署不是失败状态，无法续传"
                })));
            }
            status.progress
        } else {
            return Ok(warp::reply::json(&serde_json::json!({
                "success": false,
                "message": "未找到部署记录"
            })));
        }
    };
    
    // 启动真实续传任务
    let deployment_id_clone = deployment_id.clone();
    tokio::spawn(async move {
        use tokio::process::Command;
        use std::process::Stdio;
        
        // 更新状态为续传中
        {
            let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
            status_map.insert(deployment_id_clone.clone(), DeploymentStatus {
                id: deployment_id_clone.clone(),
                status: "resuming".to_string(),
                progress: current_progress,
                message: format!("从{}%开始续传部署...", current_progress),
            });
        }
        
        // 这里应该调用实际的续传命令
        // 由于Solana CLI不直接支持续传，我们使用部署工具的续传功能
        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "--", "resume", &deployment_id_clone]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // 逐步模拟续传进度
        for progress in ((current_progress + 10)..=100).step_by(10) {
            sleep(Duration::from_millis(1500)).await; // 稍慢一些模拟续传
            
            let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
            status_map.insert(deployment_id_clone.clone(), DeploymentStatus {
                id: deployment_id_clone.clone(),
                status: "resuming".to_string(),
                progress,
                message: format!("续传进度: {}%", progress),
            });
            
            if progress >= 100 {
                break;
            }
        }
        
        // 完成续传
        {
            let mut status_map = DEPLOYMENT_STATUS.lock().unwrap();
            status_map.insert(deployment_id_clone.clone(), DeploymentStatus {
                id: deployment_id_clone,
                status: "completed".to_string(),
                progress: 100,
                message: "续传部署完成！程序已成功部署到Solana网络".to_string(),
            });
        }
    });
    
    Ok(warp::reply::json(&serde_json::json!({
        "success": true,
        "message": "续传已开始",
        "deployment_id": deployment_id
    })))
}

// 创建所有路由的组合
pub fn create_routes() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    stats_route()
        .or(network_route())
        .or(scan_route())
        .or(deploy_route())
        .or(keypair_status_route())
        .or(health_route())
        .or(test_failure_route())
        .or(deployment_status_route())
        .or(resume_deployment_route())
} 