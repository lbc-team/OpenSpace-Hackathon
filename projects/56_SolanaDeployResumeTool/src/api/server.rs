use warp::Filter;
// use std::convert::Infallible;
use crate::api::routes;

pub async fn start_server(port: u16, rpc_url: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动Solana部署续传工具Web服务器...");
    println!("🌐 RPC端点: {}", rpc_url);
    
    // API路由
    let api = routes::create_routes();

    // 静态文件服务 - 前端页面
    let static_files = warp::path("static")
        .and(warp::fs::dir("frontend/dist"));

    // 前端页面路由
    let index = warp::path::end()
        .map(|| {
            warp::reply::html(include_str!("../../frontend/index.html"))
        });

    // WebSocket路由
    let websocket = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(handle_websocket)
        });

    // 组合所有路由
    let routes = api
        .or(websocket)
        .or(static_files)
        .or(index)
        .with(warp::cors().allow_any_origin());

    println!("✅ Web服务器已启动在端口 {}", port);
    println!("📱 前端界面: http://localhost:{}", port);
    println!("🔌 API接口: http://localhost:{}/api", port);
    println!("🔗 WebSocket: ws://localhost:{}/ws", port);

    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;

    Ok(())
}

// WebSocket连接处理
async fn handle_websocket(websocket: warp::ws::WebSocket) {
    println!("🔗 新的WebSocket连接");
    
    let (mut ws_tx, mut ws_rx) = websocket.split();
    
    // 发送初始消息
    let welcome_msg = serde_json::json!({
        "type": "connected",
        "message": "WebSocket连接已建立"
    });
    
    if let Ok(msg) = serde_json::to_string(&welcome_msg) {
        if let Err(e) = ws_tx.send(warp::ws::Message::text(msg)).await {
            println!("❌ 发送欢迎消息失败: {}", e);
            return;
        }
    }
    
    // 监听来自客户端的消息
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        println!("📨 收到WebSocket消息: {}", text);
                        // 处理客户端消息
                        if let Some(response) = handle_websocket_message(text).await {
                            if let Err(e) = ws_tx.send(warp::ws::Message::text(response)).await {
                                println!("❌ 发送回复失败: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ WebSocket错误: {}", e);
                break;
            }
        }
    }
    
    println!("🔌 WebSocket连接已关闭");
}

// 处理WebSocket消息
async fn handle_websocket_message(message: &str) -> Option<String> {
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(message) {
        match data["type"].as_str() {
            Some("ping") => {
                let pong = serde_json::json!({
                    "type": "pong",
                    "timestamp": chrono::Utc::now().timestamp()
                });
                serde_json::to_string(&pong).ok()
            }
            Some("subscribe_deployment") => {
                // 订阅部署状态更新
                let response = serde_json::json!({
                    "type": "subscription_confirmed",
                    "subscription": "deployment_updates"
                });
                serde_json::to_string(&response).ok()
            }
            _ => None,
        }
    } else {
        None
    }
}

use futures_util::{SinkExt, StreamExt}; 