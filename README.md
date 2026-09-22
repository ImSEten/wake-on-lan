# Wake-on-LAN Server

基于 Rust 的 Wake-on-LAN (网络唤醒) 服务器，提供 Web 界面和 API 接口，方便远程唤醒局域网中的设备。

## 编译

### 编译环境

- Rust (edition 2024)
- Cargo

### 编译命令

```bash
cargo build --release
```

编译完成后，可执行文件位于 `target/release/wake-on-lan-server`。

## 使用说明

### 使用步骤

**1. 配置设备信息**

在 `/var/lib/wol/mac.json` 文件中写入需要唤醒的设备的 IP、MAC 地址和主机名等信息。

示例 `mac.json` 文件：

```json
{
  "devices": [
    {
      "hostname": "nas-eth",
      "mac": "1C:7E:51:A1:50:EB",
      "ip": "192.168.99.12"
    },
    {
      "hostname": "WIN",
      "mac": "3C:7C:3F:4C:C3:CA",
      "ip": "192.168.99.3"
    },
    {
      "hostname": "Lei-WIFI",
      "mac": "40:EC:99:21:FE:E4",
      "ip": "192.168.99.14"
    }
  ]
}
```

> **注意：** 请确保 `/var/lib/wol/` 目录存在，如不存在请先创建：
> ```bash
> sudo mkdir -p /var/lib/wol
> ```

**2. 启动 Wake-on-LAN 服务**

```bash
# 使用默认参数启动（监听 [::]:10086，配置文件 /var/lib/wol/mac.json）
wake-on-lan-server start

# 自定义参数启动
wake-on-lan-server start --ip 0.0.0.0 --port 8080 --json-file /path/to/mac.json
```

启动参数说明：

| 参数 | 短选项 | 默认值 | 说明 |
|------|--------|--------|------|
| `--ip` | - | `[::]` | 服务监听 IP 地址 |
| `--port` | `-p` | `10086` | 服务监听端口 |
| `--json-file` | `-j` | `/var/lib/wol/mac.json` | 设备信息 JSON 文件路径 |

**3. 打开 Web 管理界面**

使用浏览器打开以下链接：

```
http://<服务器IP>:10086
```

例如，如果服务运行在本机，则访问 [http://localhost:10086](http://localhost:10086)。

**4. 选择并唤醒设备**

在 Web 管理页面中，选择需要 Wake-on-LAN 的设备，然后点击「唤醒设备」按钮即可远程开机。

## API 接口

| 接口 | 方法 | 说明 |
|------|------|------|
| `/wol/list` | GET | 获取所有设备列表 |
| `/wol/wake` | POST | 唤醒指定设备（通过 mac / ip / hostname 参数） |

### 唤醒设备示例

```bash
# 通过 MAC 地址唤醒
curl -X POST http://localhost:10086/wol/wake -H "Content-Type: application/json" -d '{"mac": "1C:7E:51:A1:50:EB"}'

# 通过 IP 地址唤醒
curl -X POST http://localhost:10086/wol/wake -H "Content-Type: application/json" -d '{"ip": "192.168.99.12"}'

# 通过主机名唤醒
curl -X POST http://localhost:10086/wol/wake -H "Content-Type: application/json" -d '{"hostname": "nas-eth"}'
```
