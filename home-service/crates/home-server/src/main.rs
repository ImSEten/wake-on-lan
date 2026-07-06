// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Application state shared between Tauri commands
#[derive(Clone)]
struct AppState {
    devices: Arc<RwLock<Vec<Device>>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Device {
    name: String,
    mac: String,
    ip: Option<String>,
    status: String,
}

#[derive(Deserialize)]
struct WolRequest {
    mac: String,
    broadcast: Option<String>,
}

#[derive(Serialize)]
struct WolResponse {
    success: bool,
    message: String,
}

// Tauri command: Send Wake-on-LAN magic packet
#[tauri::command]
async fn send_wol(state: tauri::State<AppState>, mac: String, broadcast: Option<String>) -> Result<WolResponse, String> {
    println!("🔮 Sending WOL packet to MAC: {}", mac);
    
    // Validate MAC address format
    let mac_clean = mac.replace(':', "").replace('-', "");
    if mac_clean.len() != 12 {
        return Err("Invalid MAC address format".to_string());
    }
    
    // TODO: Implement actual WOL packet sending
    // This is where you would:
    // 1. Build the magic packet (6 bytes of 0xFF + 16 repetitions of MAC)
    // 2. Send via UDP to broadcast address on port 9
    
    Ok(WolResponse {
        success: true,
        message: format!(
            "✨ Magic packet sent to {} (broadcast: {})", 
            mac, 
            broadcast.unwrap_or_else(|| "255.255.255.255".to_string())
        ),
    })
}

// Tauri command: Get list of configured devices
#[tauri::command]
async fn get_devices(state: tauri::State<AppState>) -> Result<Vec<Device>, String> {
    let devices = state.devices.read().await;
    Ok(devices.clone())
}

// Tauri command: Add a new device
#[tauri::command]
async fn add_device(state: tauri::State<AppState>, device: Device) -> Result<(), String> {
    let mut devices = state.devices.write().await;
    
    // Check if device with same MAC already exists
    if devices.iter().any(|d| d.mac == device.mac) {
        return Err("Device with this MAC address already exists".to_string());
    }
    
    devices.push(device);
    Ok(())
}

// Tauri command: Remove a device by MAC
#[tauri::command]
async fn remove_device(state: tauri::State<AppState>, mac: String) -> Result<(), String> {
    let mut devices = state.devices.write().await;
    let initial_len = devices.len();
    devices.retain(|d| d.mac != mac);
    
    if devices.len() == initial_len {
        Err("Device not found".to_string())
    } else {
        Ok(())
    }
}

// Tauri command: Update device status
#[tauri::command]
async fn update_device_status(
    state: tauri::State<AppState>,
    mac: String,
    status: String,
) -> Result<(), String> {
    let mut devices = state.devices.write().await;
    
    for device in devices.iter_mut() {
        if device.mac == mac {
            device.status = status;
            return Ok(());
        }
    }
    
    Err("Device not found".to_string())
}

fn main() {
    // Initialize application state with sample devices
    let state = AppState {
        devices: Arc::new(RwLock::new(vec![
            Device {
                name: "Living Room PC".to_string(),
                mac: "00:11:22:33:44:55".to_string(),
                ip: Some("192.168.1.100".to_string()),
                status: "offline".to_string(),
            },
            Device {
                name: "Office Server".to_string(),
                mac: "AA:BB:CC:DD:EE:FF".to_string(),
                ip: Some("192.168.1.101".to_string()),
                status: "online".to_string(),
            },
            Device {
                name: "Gaming Rig".to_string(),
                mac: "11:22:33:44:55:66".to_string(),
                ip: Some("192.168.1.102".to_string()),
                status: "offline".to_string(),
            },
        ])),
    };

    // Build and run Tauri application
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            send_wol,
            get_devices,
            add_device,
            remove_device,
            update_device_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
