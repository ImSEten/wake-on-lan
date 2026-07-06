# Home Server - Wake on LAN 快速开始指南

## 🚀 5分钟快速上手

### 前提条件

#### 1. Rust工具链
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # 应该显示 rustc 1.70+
```

#### 2. Tauri系统依赖

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

**Fedora/RHEL/CentOS:**
```bash
sudo dnf install -y webkit2gtk3-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel
```

**Arch Linux:**
```bash
sudo pacman -S webkit2gtk gtk3 libappindicator-gtk3 librsvg
```

### 编译和运行

#### 方法1: 直接运行（推荐）

```bash
cd /home/10307750@zte.intra/work/codes/github.com/home-service/home-service

# 运行Tauri桌面应用
cargo run --package home-server
```

这将：
1. 编译所有依赖
2. 打开一个900x700的窗口
3. 显示Wake-on-LAN控制界面

#### 方法2: 先编译后运行

```bash
# 编译release版本（更快，更小）
cargo build --release --package home-server

# 运行编译好的程序
./target/release/home-server
```

### 使用界面

#### 发送WOL包
1. 在"MAC Address"输入框中输入MAC地址（如：`00:11:22:33:44:55`）
2. （可选）在"Broadcast Address"中输入广播地址（默认：`255.255.255.255`）
3. 点击"📡 Send Magic Packet"按钮
4. 等待成功消息

#### 管理设备

**查看设备列表：**
- 界面会自动加载预置的3个示例设备
- 每个设备显示名称、MAC、IP和状态

**添加新设备：**
1. 滚动到"➕ Add New Device"区域
2. 填写设备名称（必填）
3. 填写MAC地址（必填，格式：`00:11:22:33:44:55`）
4. 填写IP地址（可选）
5. 点击"Add Device"按钮

**唤醒设备：**
- 点击设备卡片右侧的"⚡ Wake"按钮
- 或使用主界面的WOL功能手动输入MAC

**删除设备：**
- 点击设备卡片右侧的"🗑️ Remove"按钮
- 确认删除操作

### 使用命令行客户端

```bash
# 发送WOL包
cargo run --package home-client -- wol --mac 00:11:22:33:44:55

# 指定广播地址
cargo run --package home-client -- wol --mac 00:11:22:33:44:55 --broadcast 192.168.1.255

# 列出设备
cargo run --package home-client -- list

# 检查服务器状态
cargo run --package home-client -- status

# 交互式模式（WebSocket）
cargo run --package home-client -- --server-url http://localhost:3000
```

## 🔍 故障排查

### 问题1: 编译错误 "package not found"

**症状：**
```
error: package `tauri v2.x.x` cannot be built because it requires rustc 1.70 or newer
```

**解决：**
```bash
rustup update
rustc --version  # 确保 >= 1.70
```

### 问题2: 缺少系统库

**症状：**
```
Package 'gtk+-3.0', required by 'virtual:world', not found
```

**解决：**
安装对应的系统依赖（见上方"前提条件"部分）

### 问题3: 窗口无法打开

**症状：**
程序编译成功但窗口不显示

**可能原因：**
- Wayland/X11显示服务器问题
- 缺少图形驱动

**解决：**
```bash
# 尝试使用X11
export GDK_BACKEND=x11
cargo run --package home-server

# 或检查显示服务器
echo $WAYLAND_DISPLAY
echo $DISPLAY
```

### 问题4: WOL按钮点击无反应

**症状：**
点击按钮但没有反馈

**检查：**
1. 打开开发者工具（Ctrl+Shift+I）
2. 查看Console标签是否有错误
3. 检查MAC地址格式是否正确

**常见错误：**
- MAC地址格式不正确（应该是`XX:XX:XX:XX:XX:XX`或`XXXXXXXXXXXX`）
- 网络权限问题（需要root权限发送原始UDP包）

## 📝 下一步

### 实现实际WOL功能

目前的WOL按钮返回模拟成功响应。要实际发送魔法包，需要：

1. **添加socket2依赖**到`crates/home-server/Cargo.toml`:
```toml
[dependencies]
socket2 = "0.5"
```

2. **更新`send_wol`命令**（见PROJECT_COMPLETE.md中的代码示例）

3. **注意权限**：发送原始UDP包可能需要root权限

### 添加HTTP服务器

要让外部通过浏览器访问界面：

1. **添加Axum依赖**:
```toml
axum = "0.7"
tower-http = { version = "0.5", features = ["cors", "fs"] }
```

2. **在main.rs中启动HTTP服务器**（见PROJECT_COMPLETE.md）

### 配置HTTPS

1. **生成证书**:
```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

2. **添加tokio-rustls依赖**
3. **配置TLS监听器**

## 🎯 验证清单

完成以下检查确保一切正常：

- [ ] Rust版本 >= 1.70
- [ ] 系统依赖已安装
- [ ] `cargo build --package home-server` 成功
- [ ] 窗口正常显示
- [ ] Wake-on-LAN按钮可点击
- [ ] 设备列表正确显示
- [ ] 可以添加新设备
- [ ] 可以删除设备
- [ ] MAC地址验证有效
- [ ] `cargo build --package home-client` 成功
- [ ] CLI工具可以运行

## 📞 获取帮助

如果遇到问题：

1. 查看`PROJECT_COMPLETE.md`了解详细实现
2. 查看`CHECKLIST.md`确认所有组件
3. 检查Cargo.lock中的依赖版本
4. 清理并重新编译：`cargo clean && cargo build`

---

**祝使用愉快！** 🎉

如有问题，请查看相关文档或联系开发者。
