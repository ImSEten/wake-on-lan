use crate::ip_manager::NetDevice;
use serde::Deserialize;
use std::net::Ipv4Addr;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// 扫描局域网中的所有IP、MAC和hostname
#[derive(Debug)]
pub struct NetworkScanner {
    network_devices: Mutex<Vec<NetworkDevice>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkDevice {
    pub ip: Ipv4Addr,
    pub mac: String,
    pub hostname: Option<String>,
}

/// 控制实际并发 fork 的进程数，避免一次性拉起上百个子进程导致系统抖动
const CONCURRENCY_LIMIT: usize = 32;

/// 扫描单个 IP：ping 判断在线 -> 取 MAC -> 取主机名（带超时）
async fn scan_one(ip: Ipv4Addr) -> Option<NetworkDevice> {
    let t0 = Instant::now();
    // ping 检查是否在线（-c1 -W1：发1包，最多等1秒）
    if !ping_device(ip).await {
        return None;
    }

    let t1 = Instant::now();
    // arp 缓存已在 ping 后命中，瞬时返回
    let mac = get_mac_address(ip).ok().flatten()?;

    // 主机名解析可能很慢（nmap），加超时避免拖慢整体
    let t2 = Instant::now();
    let hostname =
        match tokio::time::timeout(Duration::from_secs(2), async { get_hostname(ip) }).await {
            Ok(ok) => ok.ok(),
            Err(_) => None, // 超时
        };

    Some(NetworkDevice { ip, mac, hostname })
}

/// 使用ping命令检查设备是否在线
async fn ping_device(ip: Ipv4Addr) -> bool {
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg("0.5")
        .arg(ip.to_string())
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 获取指定IP的MAC地址
fn get_mac_address(ip: Ipv4Addr) -> Result<Option<String>, Box<dyn std::error::Error + Send>> {
    // 使用arp命令获取MAC地址（ping 后 arp 缓存已命中，瞬时返回）
    let output = Command::new("arp")
        .arg("-n")
        .arg(ip.to_string())
        .output()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains(&ip.to_string()) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Ok(Some(parts[2].to_string()));
                }
            }
        }
    }

    Ok(None)
}

/// 获取指定IP的主机名
fn get_hostname(ip: Ipv4Addr) -> Result<String, Box<dyn std::error::Error + Send>> {
    // 优先使用 getent hosts（Linux 自带，输出形如 "192.168.99.1\t_gateway"）
    let output = Command::new("getent")
        .arg("hosts")
        .arg(ip.to_string())
        .output()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // parts[0] 是 IP，parts[1] 是主机名（若存在）
            if parts.len() >= 2 && parts[0] == ip.to_string() {
                return Ok(parts[1].to_string());
            }
        }
    }

    // getent 失败时回退到 nslookup（输出形如 "...name = _gateway."，注意 "name" "=" 之间是制表符）
    let output = Command::new("nslookup")
        .arg(ip.to_string())
        .output()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("name = ") {
                // split_whitespace: ["arpa", "name", "=", "_gateway."] -> 取 parts[3]
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 && parts[1] == "name" && parts[2] == "=" {
                    return Ok(parts[3].trim_matches('"').to_string());
                }
            }
        }
    }

    Err(Box::new(std::io::Error::other("Failed to get hostname")))
}

impl NetworkScanner {
    pub fn new() -> Self {
        NetworkScanner {
            network_devices: Mutex::new(Vec::new()),
        }
    }

    /// 扫描局域网中的所有设备
    pub async fn scan_network(
        &self,
        net_devices: &[NetDevice],
    ) -> Result<Vec<NetworkDevice>, Box<dyn std::error::Error + Send>> {
        let mut result = Vec::new();
        let overall_start = Instant::now();

        // 控制实际并发 fork 的进程数，避免一次性拉起上百个子进程导致抖动
        let concurrency = Arc::new(Semaphore::new(CONCURRENCY_LIMIT));

        for device in net_devices {
            println!("Scanning device: {:?}", device);
            // 获取设备的网络范围
            for ip in &device.ips {
                let ip_str = ip.to_string();
                // 处理带有子网掩码的IP地址格式，如 10.10.99.2/24
                let ip_part = if let Some(pos) = ip_str.find('/') {
                    &ip_str[..pos]
                } else {
                    &ip_str
                };

                if let Ok(ip) = ip_part.parse::<Ipv4Addr>() {
                    // 获取网络前缀，假设为24位子网掩码
                    let network = Ipv4Addr::new(ip.octets()[0], ip.octets()[1], ip.octets()[2], 0);

                    // 派发并发扫描任务（实际进程数受 Semaphore 限制）
                    let net_start = Instant::now();
                    let mut tasks = Vec::new();
                    for i in 1..255 {
                        let target_ip = Ipv4Addr::new(
                            network.octets()[0],
                            network.octets()[1],
                            network.octets()[2],
                            i,
                        );
                        if target_ip == ip {
                            continue; // 跳过自身
                        }

                        let permit = Arc::clone(&concurrency).acquire_owned().await.unwrap();
                        tasks.push(tokio::spawn(async move {
                            let device = scan_one(target_ip).await;
                            drop(permit);
                            device
                        }));
                    }

                    // 收集扫描结果
                    for task in tasks {
                        if let Ok(Some(device)) = task.await {
                            result.push(device);
                        }
                    }
                } else {
                    println!("Invalid IP address: {}", ip_str);
                }
            }
        }

        // 更新网络设备列表
        let mut devices = self.network_devices.lock().await;
        *devices = result.clone();

        Ok(result)
    }

    /// 获取扫描结果
    pub async fn get_scan_results(&self) -> Vec<NetworkDevice> {
        self.network_devices.lock().await.clone()
    }
}

impl Default for NetworkScanner {
    fn default() -> Self {
        Self::new()
    }
}
