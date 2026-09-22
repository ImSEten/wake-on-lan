use std::net::{Ipv4Addr, UdpSocket};
use std::str::FromStr;
use std::vec;

use crate::ip_manager::IpManager;

/// 通过IP地址唤醒设备
///
/// # 参数
/// * `ip_manager` - IP管理器，用于IP到MAC地址的映射
/// * `ip_address` - 目标IP地址
///
/// # 返回值
/// * `Result<()>` - 成功返回Ok，失败返回Error
pub async fn wol_ip(ip_manager: &IpManager, ip_address: &str) -> Result<(), String> {
    // 1. 检查ip参数是否是ip地址
    let ip = Ipv4Addr::from_str(ip_address).map_err(|_| format!("无效的IP地址: {}", ip_address))?;

    // 2. 在ip_manager中匹配ip地址，获取mac地址
    let mac_address = ip_manager
        .get_mac_by_ip(&ip)
        .await
        .ok_or_else(|| format!("无法找到IP {} 对应的MAC地址", ip_address))?;

    // 3. 向该mac地址发送wake-on-lan魔术包
    send_wol_packet(&mac_address)?;

    // 4. 返回成功
    Ok(())
}

/// 通过MAC地址直接唤醒设备
///
/// # 参数
/// * `mac_address` - 目标MAC地址
///
/// # 返回值
/// * `Result<()>` - 成功返回Ok，失败返回Error
pub fn wol_mac(mac_address: &str) -> Result<(), String> {
    // 1. 向该mac地址发送wake-on-lan魔术包
    send_wol_packet(mac_address)
}

/// 通过主机名唤醒设备
///
/// # 参数
/// * `ip_manager` - IP管理器实例
/// * `hostname` - 目标主机名
/// * `port` - 端口号，默认为9
///
/// # 返回值
/// * `Result<()>` - 成功返回Ok，失败返回Error
pub async fn wol_hostname(ip_manager: &IpManager, hostname: &str) -> Result<(), String> {
    // 1. 使用IP管理器获取MAC地址
    let mac_address = ip_manager
        .get_mac_by_hostname(hostname)
        .await
        .ok_or_else(|| format!("无法找到主机名 {} 对应的MAC地址", hostname))?;

    // 2. 向该mac地址发送wake-on-lan魔术包
    send_wol_packet(&mac_address)?;

    // 3. 返回成功
    Ok(())
}

/// 发送Wake-on-LAN魔术包
///
/// # 参数
/// * `mac_address` - 目标MAC地址
///
/// # 返回值
/// * `Result<String>` - 成功返回Ok，失败返回错误信息
fn send_wol_packet(mac_address: &str) -> Result<(), String> {
    // 验证MAC地址格式
    validate_mac_address(mac_address)?;

    // 创建魔术包: 6字节的0xFF followed by 16次重复的MAC地址
    // 添加6字节的0xFF
    let mut magic_packet = vec![0xFF; 6];

    // 添加16次重复的MAC地址
    let mac_bytes = parse_mac_address(mac_address)?;
    for _ in 0..16 {
        magic_packet.extend_from_slice(&mac_bytes);
    }

    // 创建UDP套接字，广播到255.255.255.255:9
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("创建UDP套接字失败: {}", e))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("设置广播失败: {}", e))?;

    // 发送魔术包
    let broadcast_addr = "255.255.255.255:9";
    socket
        .send_to(&magic_packet, broadcast_addr)
        .map_err(|e| format!("发送魔术包失败: {}", e))?;

    Ok(())
}

/// 验证MAC地址格式
///
/// # 参数
/// * `mac_address` - 要验证的MAC地址
///
/// # 返回值
/// * `Result<()>` - 格式正确返回Ok，否则返回错误信息
fn validate_mac_address(mac_address: &str) -> Result<(), String> {
    parse_mac_address(mac_address)?;
    Ok(())
}

/// 解析MAC地址为字节数组
///
/// # 参数
/// * `mac_address` - 要解析的MAC地址
///
/// # 返回值
/// * `Result<[u8; 6]>` - 成功返回6字节的MAC地址数组，否则返回错误信息
fn parse_mac_address(mac_address: &str) -> Result<[u8; 6], String> {
    // 移除可能的分隔符（冒号或连字符）
    let normalized_mac = mac_address.to_uppercase().replace(":", "").replace("-", "");

    // 检查长度是否为12个字符
    if normalized_mac.len() != 12 {
        return Err(format!("无效的MAC地址长度: {}", mac_address));
    }

    // 尝试解析为6个字节
    let mut bytes = [0u8; 6];
    for (i, chunk) in normalized_mac.as_bytes().chunks(2).enumerate() {
        let byte_str =
            std::str::from_utf8(chunk).map_err(|_| format!("无效的MAC地址: {}", mac_address))?;
        bytes[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| format!("无效的MAC地址: {}", mac_address))?;
    }

    Ok(bytes)
}
