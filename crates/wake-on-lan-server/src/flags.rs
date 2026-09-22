const IP: &str = "[::]";
const PORT: u16 = 10086;
const JSON_FILE: &str = "/var/lib/wol/mac.json";

#[derive(clap::Parser)]
#[command(name = "AutoServer")]
#[command(about = "AutoServer is my own server", long_about = None)]
pub struct Flags {
    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// 子命令枚举
#[derive(clap::Subcommand)]
pub enum Commands {
    /// 启动服务
    #[command(name = "start", about = "Start the server")]
    Start {
        /// server listening ip addr
        #[arg(long, help = "server listening ip addr", default_value = IP)]
        ip: String,

        /// server listening ip port
        #[arg(short, long, default_value_t = PORT, help = "server listening ip port")]
        port: u16,

        /// mac ip hostname map file path
        #[arg(short, long, default_value = JSON_FILE, help = "server mac ip hostname map file path")]
        json_file: String,
    },
    /// 停止服务
    #[command(name = "stop", about = "Stop the server")]
    Stop,
    // /// 扫描局域网设备
    // #[command(name = "scan", about = "Scan network devices")]
    // Scan,
}
