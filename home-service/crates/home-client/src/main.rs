use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::connect_async;

#[derive(Debug)]
struct Args {
    server_url: String,
    command: Option<Commands>,
}

#[derive(Debug)]
enum Commands {
    Wol { mac: String, broadcast: String },
    List,
    Status,
}

impl Args {
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        
        // Simple argument parsing (replacing clap)
        let mut server_url = "http://localhost:3000".to_string();
        let mut command = None;
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--server-url" | "-s" => {
                    if i + 1 < args.len() {
                        server_url = args[i + 1].clone();
                        i += 2;
                    } else {
                        eprintln!("Error: --server-url requires a value");
                        std::process::exit(1);
                    }
                }
                "wol" => {
                    let mut mac = String::new();
                    let mut broadcast = "255.255.255.255".to_string();
                    
                    i += 1;
                    while i < args.len() && !args[i].starts_with('-') {
                        if mac.is_empty() {
                            mac = args[i].clone();
                        } else if broadcast == "255.255.255.255" {
                            broadcast = args[i].clone();
                        }
                        i += 1;
                    }
                    
                    // Parse flags for wol command
                    while i < args.len() {
                        match args[i].as_str() {
                            "--mac" | "-m" => {
                                if i + 1 < args.len() {
                                    mac = args[i + 1].clone();
                                    i += 2;
                                }
                            }
                            "--broadcast" | "-b" => {
                                if i + 1 < args.len() {
                                    broadcast = args[i + 1].clone();
                                    i += 2;
                                }
                            }
                            _ => break,
                        }
                    }
                    
                    if mac.is_empty() {
                        eprintln!("Error: wol command requires --mac argument");
                        std::process::exit(1);
                    }
                    
                    command = Some(Commands::Wol { mac, broadcast });
                }
                "list" => {
                    command = Some(Commands::List);
                    i += 1;
                }
                "status" => {
                    command = Some(Commands::Status);
                    i += 1;
                }
                "--help" | "-h" => {
                    println!("Home Client - Wake on LAN Control");
                    println!();
                    println!("USAGE:");
                    println!("    home-client [OPTIONS] [COMMAND]");
                    println!();
                    println!("OPTIONS:");
                    println!("    -s, --server-url <URL>    Home server URL [default: http://localhost:3000]");
                    println!("    -h, --help                Print help information");
                    println!();
                    println!("COMMANDS:");
                    println!("    wol --mac <MAC>           Send Wake-on-LAN packet");
                    println!("    list                      List available devices");
                    println!("    status                    Check server status");
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("Unknown argument: {}", args[i]);
                    std::process::exit(1);
                }
            }
        }
        
        Args { server_url, command }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct WolRequest {
    mac: String,
    broadcast: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WolResponse {
    success: bool,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Device {
    name: String,
    mac: String,
    ip: Option<String>,
    status: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::new();
    let client = Client::new();

    match &args.command {
        Some(Commands::Wol { mac, broadcast }) => {
            println!("Sending Wake-on-LAN packet to {}", mac);
            
            let request = WolRequest {
                mac: mac.clone(),
                broadcast: Some(broadcast.clone()),
            };

            let response = client
                .post(format!("{}/api/wol", args.server_url))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let wol_response: WolResponse = response.json().await?;
                println!("✓ {}", wol_response.message);
            } else {
                eprintln!("✗ Failed to send WOL packet: {}", response.status());
            }
        }

        Some(Commands::List) => {
            println!("Fetching device list...");
            
            let response = client
                .get(format!("{}/api/devices", args.server_url))
                .send()
                .await?;

            if response.status().is_success() {
                let devices: Vec<Device> = response.json().await?;
                println!("\nAvailable devices:");
                for device in devices {
                    println!(
                        "  {} - {} ({}) [{}]",
                        device.name, device.mac, 
                        device.ip.unwrap_or_else(|| "unknown".to_string()),
                        device.status
                    );
                }
            } else {
                eprintln!("✗ Failed to fetch device list: {}", response.status());
            }
        }

        Some(Commands::Status) => {
            println!("Checking server status...");
            
            let response = client
                .get(format!("{}/api/status", args.server_url))
                .send()
                .await?;

            if response.status().is_success() {
                let status: serde_json::Value = response.json().await?;
                println!("Server is online!");
                println!("Status: {:?}", status);
            } else {
                eprintln!("✗ Server is not responding: {}", response.status());
            }
        }

        None => {
            // Interactive mode with WebSocket
            println!("Connecting to home server at {}", args.server_url);
            
            let ws_url = args.server_url.replace("http", "ws").replace("https", "wss");
            let ws_url = format!("{}/ws", ws_url);
            
            match connect_async(&ws_url).await {
                Ok((mut ws_stream, _)) => {
                    println!("Connected to WebSocket!");
                    
                    // Send a ping
                    ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(
                        "{\"type\": \"ping\"}".into()
                    )).await?;
                    
                    // Listen for messages
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                println!("Received: {}", text);
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                                println!("Connection closed");
                                break;
                            }
                            Err(e) => {
                                eprintln!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to WebSocket: {}", e);
                    println!("Falling back to HTTP polling...");
                    
                    // Simple HTTP polling as fallback
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        
                        match client.get(format!("{}/api/status", args.server_url)).send().await {
                            Ok(response) if response.status().is_success() => {
                                print!(".");
                            }
                            _ => {
                                eprintln!("Server unreachable");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
