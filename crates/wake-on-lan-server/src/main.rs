use clap::{CommandFactory, Parser};
use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, Json, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

mod flags;

// 设备信息结构体，用于前端展示
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
}

// IP管理器状态
pub struct AppState {
    pub ip_manager: Arc<backend::ip_manager::IpManager>,
    pub wol_server: WOLServer,
}

// 使用 Clone trait 来确保 AppState 可以被复制
impl Clone for AppState {
    fn clone(&self) -> Self {
        AppState {
            ip_manager: self.ip_manager.clone(),
            wol_server: self.wol_server.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WOLServer {
    ip: String,
    port: u16,
    json_file: String,
}

impl WOLServer {
    pub fn new(ip: String, port: u16, json_file: String) -> Self {
        WOLServer {
            ip,
            port,
            json_file,
        }
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(32)
        .max_blocking_threads(32)
        .build()
        .expect("build tokio runtime error!");
    runtime.block_on(async_main());
}

async fn async_main() {
    let parse_flags = flags::Flags::parse();
    match parse_flags.command {
        Some(flags::Commands::Start {
            ip,
            port,
            json_file,
        }) => {
            let wol_server = WOLServer::new(ip.clone(), port, json_file.clone());
            println!(
                "service starting on {}:{}, saved_json_file: {}",
                ip, port, json_file
            );
            create_service(wol_server).await;
            println!("service exited");
        }
        Some(flags::Commands::Stop) => {
            println!("not implement!")
        }
        None => {
            if let Err(e) = flags::Flags::command().print_help() {
                println!("print_help failed {:?}", e);
            };
        }
    }
}

pub async fn create_service(wol_server: WOLServer) {
    // 创建IP管理器
    let ip_manager = Arc::new(backend::ip_manager::IpManager::new().await);

    // 启动网络监控任务
    let ip_manager_clone = ip_manager.clone();
    tokio::spawn(async move {
        let _handles = backend::ip_manager::monitor_ip(ip_manager_clone).await;
    });
    let w = wol_server.clone();

    // 创建应用状态
    let state = AppState {
        ip_manager,
        wol_server,
    };

    // 创建路由
    let app = Router::new()
        // 静态文件服务
        .route(
            "/",
            get(|| async {
                // 重定向到静态页面
                Redirect::permanent("/static/index.html")
            }),
        )
        .route(
            "/static/index.html",
            get(|| async {
                // 返回主页
                let html_content =
                    tokio::fs::read_to_string("crates/wake-on-lan-server/static/index.html")
                        .await
                        .unwrap_or_else(|_| "无法加载页面".to_string());
                Html(html_content)
            }),
        )
        // list接口：获取所有设备信息
        .route("/wol/list", get(list_devices))
        // wake-on-lan接口：唤醒指定设备
        .route("/wol/wake", post(wake_device))
        // 使用状态
        .with_state(state);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", w.ip.clone(), w.port.clone()))
        .await
        .unwrap();
    println!("Server listening on http://{}:{}", w.ip, w.port);

    axum::serve(listener, app).await.unwrap();
}

/// 获取所有设备列表
async fn list_devices(State(state): State<AppState>) -> Result<Json<Vec<DeviceInfo>>, StatusCode> {
    let ip_manager = state.ip_manager;
    let _devices = ip_manager.get_network_devices().await;
    let devices = backend::ip_manager::load_ip_mac_mapping(state.wol_server.json_file.as_str());
    println!("devices: {:?}", devices);

    let device_infos: Vec<DeviceInfo> = devices
        .into_iter()
        .map(|device| DeviceInfo {
            ip: device.ip.to_string(),
            mac: device.mac,
            hostname: device.hostname,
        })
        .collect();

    Ok(Json(device_infos))
}

/// 唤醒设备接口
#[derive(Debug, Deserialize)]
struct WakeRequest {
    pub mac: Option<String>,
    pub ip: Option<String>,
    pub hostname: Option<String>,
}

async fn wake_device(
    State(state): State<AppState>,
    Json(req): Json<WakeRequest>,
) -> Result<Json<String>, StatusCode> {
    let ip_manager = state.ip_manager;

    // 根据不同的参数调用不同的唤醒函数
    if let Some(mac) = req.mac {
        match backend::wol::wol_mac(&mac) {
            Ok(_) => Ok(Json(format!("成功发送唤醒包到MAC地址: {}", mac))),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else if let Some(ip) = req.ip {
        match backend::wol::wol_ip(&ip_manager, &ip).await {
            Ok(_) => Ok(Json(format!("成功唤醒IP地址: {}", ip))),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else if let Some(hostname) = req.hostname {
        match backend::wol::wol_hostname(&ip_manager, &hostname).await {
            Ok(_) => Ok(Json(format!("成功唤醒主机: {}", hostname))),
            Err(_e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
