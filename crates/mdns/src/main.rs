use mdns_sd::ServiceDaemon;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    let target_ip = "192.168.99.105";
    let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");

    println!("🔍 Searching for hostname with IP {target_ip} ...");

    // 尝试通过IP地址解析主机名
    let result = timeout(Duration::from_secs(5), async {
        // 尝试通过IP地址解析主机名
        // 首先尝试反向DNS查询
        let hostname = format!(
            "{}.in-addr.arpa.",
            target_ip.split('.').collect::<Vec<_>>().join(".")
        );

        // 尝试解析主机名
        let receiver = match mdns.resolve_hostname(&format!("{}local.", hostname), None) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("⚠️  Failed to resolve hostname: {e}");
                return None;
            }
        };

        // 设置超时，避免无限等待
        let timeout_result = timeout(Duration::from_secs(3), async {
            while let Ok(event) = receiver.recv_async().await {
                if let mdns_sd::HostnameResolutionEvent::AddressesFound(name, addresses) = event {
                    // 检查找到的地址是否匹配目标IP
                    for addr in addresses {
                        if addr.to_string() == target_ip {
                            return Some(name.clone().trim_end_matches(".local").to_string());
                        }
                    }
                }
            }
            None
        })
        .await;

        match timeout_result {
            Ok(Some(hostname)) => Some(hostname),
            _ => None,
        }
    })
    .await;

    match result {
        Ok(Some(hostname)) => println!("✅ Hostname: {hostname}"),
        Ok(None) => println!("❌ Failed to resolve hostname for {target_ip}"),
        Err(_) => println!("⏰ Timeout after 5s"),
    }

    mdns.shutdown().unwrap();
}
