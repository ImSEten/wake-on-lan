# Home Service - Wake on LAN

这是一个使用Rust和Tauri框架开发的Home Server应用程序，提供Wake-on-LAN功能。

## 项目结构

```
home-service/
├── crates/
│   ├── home-server/          # Tauri桌面应用程序 + Web服务器
│   │   ├── src/
│   │   │   └── main.rs       # 主程序入口
│   │   ├── dist/             # 前端静态文件
│   │   │   └── index.html    # Web界面
│   │   ├── tauri.conf.json   # Tauri配置文件
│   │   └── Cargo.toml
│   ├── home-client/          # 命令行客户端
│   │   ├── src/
│   │   │   └── main.rs       # 客户端程序
│   │   └── Cargo.toml
│   └── common/               # 共享代码
│       └── src/
│           ├── lib.rs
│           ├── command.rs
│           └── services.rs
├── vendor/                   # Rust依赖
├── Cargo.toml                # 工作区配置
└── Cargo.lock
```

## 功能特性

### Home Server
- **Tauri桌面应用**: 提供图形化界面用于发送WOL魔法包
- **Web服务器**: 通过HTTP/HTTPS提供REST API
- **WebSocket支持**: 实时通信
- **CORS支持**: 允许跨域请求

### Home Client
- **命令行工具**: 通过命令行发送WOL包
- **HTTP API客户端**: 与home-server交互
- **WebSocket客户端**: 实时连接服务器
- **设备管理**: 列出和管理可用设备

## 安装和构建

### 前提条件
- Rust工具链 (rustc, cargo)
- Tauri依赖 (详见[Tauri文档](https://tauri.app/v1/guides/getting-started/prerequisites))

### 构建步骤

1. **克隆仓库**
```bash
git clone <repository-url>
cd home-service
```

2. **下载依赖**
```bash
cargo vendor --versioned-dirs
```

3. **构建项目**
```bash
# 构建所有组件
cargo build --workspace

# 或者单独构建
cargo build --package home-server
cargo build --package home-client
```

4. **运行**
```bash
# 运行home-server (桌面应用 + Web服务器)
cargo run --package home-server

# 运行home-client (命令行工具)
cargo run --package home-client -- --help
```

## 使用方法

### Home Server

启动后，home-server会：
1. 打开一个Tauri桌面窗口
2. 在端口3000上启动HTTP服务器
3. 提供以下API端点：
   - `POST /api/wol` - 发送WOL魔法包
   - `GET /api/devices` - 获取设备列表
   - `GET /api/status` - 获取服务器状态

### Home Client

```bash
# 发送WOL包到指定MAC地址
cargo run --package home-client -- wol --mac 00:11:22:33:44:55

# 指定广播地址
cargo run --package home-client -- wol --mac 00:11:22:33:44:55 --broadcast 192.168.1.255

# 列出可用设备
cargo run --package home-client -- list

# 检查服务器状态
cargo run --package home-client -- status

# 交互式模式（默认）
cargo run --package home-client -- --server-url http://localhost:3000
```

## API文档

### POST /api/wol

发送Wake-on-LAN魔法包。

**请求体:**
```json
{
  "mac": "00:11:22:33:44:55",
  "broadcast": "255.255.255.255"
}
```

**响应:**
```json
{
  "success": true,
  "message": "WOL packet sent to 00:11:22:33:44:55 (broadcast: 255.255.255.255)"
}
```

### GET /api/devices

获取配置的设备列表。

**响应:**
```json
[
  {
    "name": "Living Room PC",
    "mac": "00:11:22:33:44:55",
    "ip": "192.168.1.100",
    "status": "offline"
  }
]
```

### GET /api/status

获取服务器状态。

**响应:**
```json
{
  "status": "online",
  "version": "0.1.0"
}
```

## 技术栈

- **后端**: Rust, Tokio, Axum
- **前端**: HTML5, CSS3, JavaScript (原生)
- **桌面框架**: Tauri v2
- **网络**: HTTP/HTTPS, WebSocket
- **协议**: Wake-on-LAN (WOL)

## 开发说明

### 添加新的WOL实现

目前WOL功能只是返回成功响应。要实际实现WOL魔法包发送，需要：

1. 使用`socket2`或`tokio::net::UdpSocket`创建UDP套接字
2. 构建WOL魔法包（6字节0xFF + 16次重复的MAC地址）
3. 发送到指定的广播地址

示例代码框架已在`main.rs`中标记为TODO。

### HTTPS配置

要启用HTTPS支持，需要：
1. 生成或获取SSL证书
2. 使用`tokio-rustls`或`tokio-native-tls`配置TLS
3. 在`main.rs`中取消注释HTTPS服务器部分

## 许可证

本项目采用MIT许可证。

## 贡献

欢迎提交Issue和Pull Request！
