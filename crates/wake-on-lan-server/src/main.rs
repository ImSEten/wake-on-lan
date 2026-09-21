use clap::{CommandFactory, Parser};
use std::sync::Arc;

mod flags;
pub struct WOLServer {
    ip: String,
    port: u16,
}

impl WOLServer {
    pub fn new(ip: String, port: u16) -> Self {
        WOLServer { ip, port }
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
    // TODO: let flags = flags.parse(); // flags is Options
    let parse_flags = flags::Flags::parse();
    match parse_flags.command {
        Some(flags::Commands::Start { ip, port }) => {
            let _wol_server = WOLServer::new(ip.clone(), port);
            println!("service starting...");
            create_service().await;
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

pub async fn create_service() {
    let ip_manager = Arc::new(backend::ip_manager::IpManager::new().await);
    let task_handles = backend::ip_manager::monitor_ip(ip_manager).await;
    for task_handle in task_handles {
        match task_handle.await {
            Ok(_) => {}
            Err(e) => {
                println!("task handle error: {:?}", e);
            }
        }
    }
}
