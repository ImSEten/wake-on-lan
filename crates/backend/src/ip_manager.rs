use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use serde::Deserialize;
use tokio::{sync::Mutex, task::JoinHandle};

#[derive(Debug, Clone)]
pub struct NetDevice {
    pub device_name: String,
    pub mac: String,
    pub ips: Vec<Ipv4Addr>,
}

#[derive(Debug)]
pub struct IpManager {
    pub net_devices: Mutex<Vec<NetDevice>>,
    pub network_scanner: Arc<super::network_scanner::NetworkScanner>,
    pub net_neighbor: Arc<Mutex<Vec<super::network_scanner::NetworkDevice>>>,
}

impl IpManager {
    pub async fn new() -> Self {
        let scanner = Arc::new(super::network_scanner::NetworkScanner::new());
        IpManager {
            net_devices: Mutex::new(get_local_net_info().await.unwrap_or_default()),
            network_scanner: scanner.clone(),
            // net_neighbor: Mutex::new(read_net_devices(&path).await),
            net_neighbor: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 扫描局域网中的所有设备
    pub async fn scan_network_devices(
        &self,
    ) -> Result<Vec<super::network_scanner::NetworkDevice>, Box<dyn std::error::Error + Send>> {
        // 获取本机网络接口信息
        let local_devices = self.net_devices.lock().await.clone();
        println!("local_devices: {:?}", local_devices);

        // 扫描局域网中的设备
        // self.network_scanner.scan_network(&local_devices).await
        match self.network_scanner.scan_network(&local_devices).await {
            Ok(devices) => {
                println!("Found {} devices:", devices.len());
                for device in devices.clone() {
                    // 如果设备没有 hostname，尝试从 ip-mac.json 中查找
                    let hostname = if let Some(h) = device.hostname {
                        h
                    } else {
                        let config_file = {
                            let exe_dir =
                                std::env::current_exe().expect("Failed to get executable path");
                            let exe_dir = exe_dir
                                .parent()
                                .expect("Failed to get executable directory");
                            exe_dir.join("ip-mac.json").to_string_lossy().to_string()
                        };
                        let config_devices = load_ip_mac_mapping(&config_file);
                        get_hostname_from_config_ip(&device.ip, &config_devices)
                            .unwrap_or_else(|| "Unknown".to_string())
                    };

                    println!(
                        "IP: {}, MAC: {}, Hostname: {}",
                        device.ip, device.mac, hostname
                    );
                }
                {
                    let mut net_neighbor = self.net_neighbor.lock().await;
                    *net_neighbor = devices.clone();
                }
                Ok(devices)
            }
            Err(e) => {
                println!("Error scanning network: {}", e);
                Err(e)
            }
        }
    }

    /// 获取网络扫描结果
    pub async fn get_network_devices(&self) -> Vec<super::network_scanner::NetworkDevice> {
        // self.network_scanner.get_scan_results().await
        self.net_neighbor.lock().await.to_vec()
    }

    pub async fn get_mac_by_hostname(&self, hostname: &str) -> Option<String> {
        let devices = self.get_network_devices().await;
        for device in devices {
            if device.hostname == Some(hostname.to_string()) {
                return Some(device.mac);
            }
        }
        None
    }

    pub async fn get_mac_by_ip(&self, ip: &Ipv4Addr) -> Option<String> {
        let devices = self.get_network_devices().await;
        for device in devices {
            if device.ip == *ip {
                return Some(device.mac);
            }
        }
        None
    }
}

#[cfg(target_os = "linux")]
pub async fn get_local_net_info() -> std::io::Result<Vec<NetDevice>> {
    let devices_name = get_net_devices().await;
    let mut args = vec!["addr".to_string(), "show".to_string()];
    let mut net_devices: Vec<NetDevice> = Vec::new();
    for device_name in devices_name {
        if device_name == "lo"
            || device_name == "docker0"
            || device_name == "lxcbr0"
            || device_name == "virbr0"
            || device_name == "easytier0"
            || device_name.contains("veth")
            || device_name.contains("br-")
            || device_name.contains("veth")
        {
            continue;
        }
        args.push(device_name.clone());
        let (stdout, _stderr) =
            common::command::Command::run("ip".to_string(), args.clone()).await?;
        let ips = parse_ip_addr(stdout.as_str());
        let mac = parse_mac_addr(stdout.as_str());
        net_devices.push(NetDevice {
            device_name,
            mac,
            ips,
        });
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
    parse_net_devices(stdout.as_str())
}

#[cfg(target_os = "linux")]
fn parse_net_devices(ip_link_show: &str) -> Vec<String> {
    ip_link_show
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
        .collect::<Vec<String>>()
}

#[cfg(target_os = "linux")]
fn parse_ipv4_addr(ip_addr_show: &str) -> Vec<String> {
    ip_addr_show
        .split('\n')
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix("inet ")
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.split("/").next())
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
fn parse_ip_addr(ip_addr_show: &str) -> Vec<Ipv4Addr> {
    let ipv4s = parse_ipv4_addr(ip_addr_show);
    let mut ips: Vec<Ipv4Addr> = Vec::new();
    for ipv4 in ipv4s {
        println!("IP: {}", ipv4);
        let ip = Ipv4Addr::from_str(ipv4.as_str()).unwrap();

        ips.push(ip);
    }
    ips
    // let mut ipv6 = parse_ipv6_addr(ip_addr_show);
    // ip.append(&mut ipv6);
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

pub async fn monitor_ip(ip_manager: Arc<IpManager>) -> Vec<JoinHandle<std::io::Result<()>>> {
    let scanner = ip_manager.clone();
    let first_handle = tokio::spawn(async move {
        loop {
            let net_devices = get_local_net_info().await?;
            let mut devices = ip_manager.net_devices.lock().await;
            *devices = net_devices;
            tokio::time::sleep(tokio::time::Duration::from_secs(60 * 30)).await;
        }
    });

    // 定期扫描局域网设备（IP、MAC、hostname），每 30 分钟一次
    let second_handle = tokio::spawn(async move {
        loop {
            match scanner.scan_network_devices().await {
                Ok(_) => {}
                Err(e) => eprintln!("Network scan error: {}", e),
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(60 * 30)).await;
        }
    });

    vec![first_handle, second_handle]
}

/// 从 ip-mac.json 中加载设备映射（ip -> hostname）
pub fn load_ip_mac_mapping(file_path: &str) -> Vec<super::network_scanner::NetworkDevice> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Vec::new();
    }

    let content =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Failed to read {}", file_path));

    let config: Config =
        serde_json::from_str(&content).unwrap_or_else(|_| panic!("Failed to parse {}", file_path));

    config.devices
}

/// 根据 ip 从设备映射中查找 hostname
fn get_hostname_from_config_ip(
    ip: &Ipv4Addr,
    devices: &[super::network_scanner::NetworkDevice],
) -> Option<String> {
    devices
        .iter()
        .find(|device| device.ip == *ip)
        .and_then(|d| d.hostname.clone())
}

#[derive(Debug, Deserialize)]
struct Config {
    devices: Vec<super::network_scanner::NetworkDevice>,
}

#[cfg(target_os = "windows")]
pub async fn get_local_net_info() -> std::io::Result<Vec<NetDevice>> {
    todo!();
}
