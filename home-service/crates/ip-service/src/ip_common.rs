use std::sync::Arc;
use tokio::{sync::Mutex, task::JoinHandle};

use crate::ip_client::Client;

use service_protos::proto_ip_service::NetDevice;

pub struct IpService {
    pub net_devices: Mutex<Vec<NetDevice>>,
}

impl IpService {
    pub async fn new() -> Self {
        IpService {
            net_devices: Mutex::new(get_net_info().await.unwrap_or_default()),
        }
    }
}

#[cfg(target_os = "linux")]
pub async fn get_net_info() -> std::io::Result<Vec<NetDevice>> {
    let devices = get_net_devices().await;
    let mut args = vec!["addr".to_string(), "show".to_string()];
    let mut net_devices: Vec<NetDevice> = Vec::new();
    for device in devices {
        args.push(device.clone());
        let (stdout, _stderr) =
            common::command::Command::run("ip".to_string(), args.clone()).await?;
        let ips = parse_ip_addr(stdout.as_str());
        let mac = parse_mac_addr(stdout.as_str());
        net_devices.push(NetDevice { device, mac, ips });
        args.pop();
    }
    Ok(net_devices)
}

#[cfg(target_os = "linux")]
async fn get_net_devices() -> Vec<String> {
    let (stdout, _stderr) = common::command::Command::run(
        "ip".to_string(),
        vec!["link".to_string(), "show".to_string()],
    )
    .await
    .unwrap();
    let devices = parse_net_devices(stdout.as_str());
    devices
}

#[cfg(target_os = "linux")]
fn parse_net_devices(ip_link_show: &str) -> Vec<String> {
    let net_devices = ip_link_show
        .split('\n')
        .filter_map(|line| {
            let line = line.trim_start();
            match line.strip_prefix("link") {
                Some(_) => None,
                None => {
                    let mut iter = line.split(": ");
                    iter.next();
                    iter.next().map(|dev| dev.to_string())
                }
            }
        })
        .collect::<Vec<String>>();
    net_devices
}

#[cfg(target_os = "linux")]
fn parse_ipv4_addr(ip_addr_show: &str) -> Vec<String> {
    ip_addr_show
        .split('\n')
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix("inet ")
                .and_then(|s| s.split_whitespace().next())
                .map(|addr| addr.to_string()) // 将 &str 转换为 String
        })
        .collect::<Vec<String>>()
}

#[cfg(target_os = "linux")]
fn parse_ipv6_addr(ip_addr_show: &str) -> Vec<String> {
    ip_addr_show
        .split('\n')
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix("inet6 ")
                .and_then(|s| s.split_whitespace().next())
                .map(|addr| addr.to_string()) // 将 &str 转换为 String
        })
        .collect::<Vec<String>>()
}

#[cfg(target_os = "linux")]
fn parse_ip_addr(ip_addr_show: &str) -> Vec<String> {
    let mut ipv4 = parse_ipv4_addr(ip_addr_show);
    let mut ipv6 = parse_ipv6_addr(ip_addr_show);
    let mut ip: Vec<String> = Vec::new();
    ip.append(&mut ipv4);
    ip.append(&mut ipv6);
    ip
}

#[cfg(target_os = "linux")]
fn parse_mac_addr(ip_addr_show: &str) -> String {
    ip_addr_show
        .split('\n')
        .filter_map(|line| {
            line.trim_start().strip_prefix("link/").and_then(|mac| {
                let mut s = mac.split_whitespace();
                s.next();
                s.next()
            })
        })
        .collect::<String>()
}

pub async fn monitor_ip(client: Arc<Mutex<Client>>) -> JoinHandle<std::io::Result<()>> {
    let ip_service = IpService::new().await;
    tokio::spawn(async move {
        loop {
            let net_devices = get_net_info().await?;
            let mut c = client.lock().await;
            if c.client.is_none() {
                println!("client connect to the server failed");
                if let Err(_e) = c.re_connect().await {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    continue;
                }
            }

            {
                let mut ip_service_lock = ip_service.net_devices.lock().await;
                if *ip_service_lock != net_devices {
                    match c.add_remote(net_devices.clone()).await {
                        Ok(_) => {
                            *ip_service_lock = net_devices;
                        }
                        Err(e) => {
                            println!("add_remote error: {:?}", e);
                            let _ = c.re_connect().await;
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    })
}

#[cfg(target_os = "windows")]
pub async fn get_net_info() {
    todo!();
}
