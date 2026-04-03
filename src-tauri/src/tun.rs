use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tun::{AbstractDevice, AsyncDevice, Configuration};

/// TUN 虚拟网卡模块 — 实现透明 TCP 代理
///
/// ## 架构概述
///
/// 本模块创建一个虚拟网络接口(TUN), 拦截所有经过该接口的 IP 数据包,
/// 并通过 SOCKS5 代理转发到目标服务器。客户端无需任何代理配置,
/// 所有流量自动通过 TUN 设备路由。
///
/// ## 数据流向
///
/// ```text
/// 客户端应用
///     ↓ (所有流量)
/// macOS 路由表(默认路由 → TUN 接口)
///     ↓
/// TUN 虚拟网卡(device.recv)
///     ↓
/// packet_processor (协议分发)
///     ├── DNS → handle_dns → 拦截/转发上游DNS
///     └── TCP → handle_tcp → SOCKS5代理 → 物理网卡 → 目标服务器
/// ```
///
/// ## 回环防护(关键!)
///
/// macOS 的 utun 设备有一个重要特性: 当我们通过 device.send() 发送响应包时,
/// 由于默认路由指向 TUN 接口, macOS 可能将这些响应包重新路由回 TUN 设备,
/// 导致 device.recv() 再次读取到我们自己发送的包。如果不加以过滤, 会形成
/// 无限循环, 导致 CPU 和网络流量飙升。
///
/// 防护措施有三层:
///
/// 1. **packet_processor 层**: 过滤源地址不是 10.0.0.2 的包
///    - 客户端发出的包源地址始终是 10.0.0.2
///    - 我们发送的响应包源地址是外部 IP(如 142.250.x.x)
///    - 因此只需丢弃源地址不是 10.0.0.2 的包即可打破回环
///
/// 2. **ACK-only 处理层**: 不对纯 ACK 包回复 ACK
///    - 如果每个 ACK 都回复 ACK, 会形成反馈循环
///    - 纯 ACK 包只更新状态, 不发送任何回复
///
/// 3. **连接清理**: SOCKS5 端关闭时立即移除连接状态
///    - 防止后续客户端迟到的 ACK 找不到连接
///    - 避免发送不必要的 RST 包
///
/// ## TCP 连接生命周期
///
/// ```text
/// 客户端                    Xnetify(TUN)              SOCKS5代理          目标服务器
///   |                          |                        |                    |
///   |--- SYN ---------------->|                        |                    |
///   |<-- SYN-ACK(立即) --------|                        |                    |
///   |                          |--- 预注册连接占位 ------|                    |
///   |                          |                        |--- 连接 --------->|
///   |                          |                        |<-- 响应 -----------|
///   |--- ACK+数据 ----------->|                        |                    |
///   |                          |-- 缓冲到 pending_data --|                    |
///   |<-- ACK -----------------|                        |                    |
///   |                          |                        |                    |
///   |                          |--- SOCKS5 握手完成 ----|                    |
///   |                          |-- 刷新 pending_data --->|--- 数据 --------->|
///   |                          |                        |                    |
///   |<== 正常数据转发 =========|<== SOCKS5 中继 ========|<== 服务器响应 =====|
///   |                          |                        |                    |
///   |--- FIN ---------------->|                        |                    |
///   |                          |--- 清理连接状态 --------|                    |
///   |<-- FIN+ACK -------------|                        |                    |
/// ```

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> TunResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn err(error: String) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl<T: Serialize, E: std::fmt::Display> From<Result<T, E>> for TunResponse<T> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => TunResponse::ok(data),
            Err(e) => TunResponse::err(e.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunConfig {
    pub device_name: String,
    pub tunnel_ip: String,
    pub tunnel_netmask: String,
    pub dns_server: String,
    pub mtu: u16,
    pub upstream_host: String,
    pub upstream_port: u16,
    pub physical_ip: String,
}

impl Default for TunConfig {
    fn default() -> Self {
        Self {
            device_name: "xnetify0".to_string(),
            tunnel_ip: "10.0.0.2".to_string(),
            tunnel_netmask: "255.255.255.0".to_string(),
            dns_server: "8.8.8.8".to_string(),
            mtu: 1500,
            upstream_host: "127.0.0.1".to_string(),
            upstream_port: 1080,
            physical_ip: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunStatus {
    pub active: bool,
    pub config: Option<TunConfig>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub error: Option<String>,
}

impl Default for TunStatus {
    fn default() -> Self {
        Self { active: false, config: None, bytes_in: 0, bytes_out: 0, error: None }
    }
}

#[derive(Debug)]
pub enum TunError {
    DeviceCreationFailed(String),
    PermissionDenied,
    AlreadyRunning,
    NotRunning,
    NetworkConfigFailed(String),
    UpstreamConnectionFailed(String),
    InvalidConfig(String),
    IoError(String),
}

impl std::fmt::Display for TunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunError::DeviceCreationFailed(msg) => write!(f, "创建设备失败: {}", msg),
            TunError::PermissionDenied => write!(f, "需要管理员/root 权限"),
            TunError::AlreadyRunning => write!(f, "TUN 设备已在运行"),
            TunError::NotRunning => write!(f, "TUN 设备未运行"),
            TunError::NetworkConfigFailed(msg) => write!(f, "网络配置失败: {}", msg),
            TunError::UpstreamConnectionFailed(msg) => write!(f, "上游连接失败: {}", msg),
            TunError::InvalidConfig(msg) => write!(f, "无效配置: {}", msg),
            TunError::IoError(msg) => write!(f, "IO 错误: {}", msg),
        }
    }
}

impl From<std::io::Error> for TunError {
    fn from(err: std::io::Error) -> Self {
        TunError::IoError(err.to_string())
    }
}

struct TcpConnState {
    server_seq: u32,
    client_ack: std::sync::Arc<std::sync::atomic::AtomicU32>,
    tx_to_socks5: Option<mpsc::Sender<Vec<u8>>>,
    pending_data: Vec<Vec<u8>>,
}

struct TunManagerInner {
    device: Option<Arc<AsyncDevice>>,
    config: Option<TunConfig>,
    running: bool,
    bytes_in: u64,
    bytes_out: std::sync::Arc<std::sync::atomic::AtomicU64>,
    processor_handle: Option<tokio::task::JoinHandle<()>>,
    tcp_conns: HashMap<String, TcpConnState>,
    sent_packet_ids: std::collections::HashSet<u64>,
}

impl Default for TunManagerInner {
    fn default() -> Self {
        Self {
            device: None, config: None, running: false,
            bytes_in: 0, bytes_out: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)), processor_handle: None,
            tcp_conns: HashMap::new(),
            sent_packet_ids: std::collections::HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct TunManager {
    inner: Arc<RwLock<TunManagerInner>>,
    engine: Arc<crate::engine::FilterEngine>,
}

impl TunManager {
    pub fn new(engine: Arc<crate::engine::FilterEngine>) -> Self {
        Self { inner: Arc::new(RwLock::new(TunManagerInner::default())), engine }
    }

    fn get_log_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_dir = PathBuf::from(home).join(".xnetify");
        let _ = fs::create_dir_all(&log_dir);
        log_dir.join("tun.log")
    }

    fn write_log(message: &str) {
        let log_path = Self::get_log_path();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] {}\n", timestamp, message);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = file.write_all(log_line.as_bytes());
        }
    }

    fn get_physical_interface_ip() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            Self::write_log("执行 get_physical_interface_ip 函数");
            let ifconfig_output = match std::process::Command::new("ifconfig").output() {
                Ok(output) => output,
                Err(e) => { Self::write_log(&format!("ifconfig 命令失败: {}", e)); return None; }
            };
            let ifconfig_stdout = String::from_utf8_lossy(&ifconfig_output.stdout);
            Self::write_log(&format!("ifconfig 输出长度: {} 字节", ifconfig_stdout.len()));
            let mut current_interface = String::new();
            for line in ifconfig_stdout.lines() {
                let is_interface_line = !line.starts_with('\t') && !line.starts_with(' ')
                    && line.contains(':') && line.len() > 1
                    && line.chars().next().map_or(false, |c| c.is_alphanumeric());
                if is_interface_line && !line.contains("utun") && !line.contains("lo0") {
                    if let Some(colon_pos) = line.find(':') {
                        current_interface = line[..colon_pos].to_string();
                        Self::write_log(&format!("检查接口: {}", current_interface));
                    }
                } else if !current_interface.is_empty() && line.trim().starts_with("inet ") {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        let ip = parts[1];
                        Self::write_log(&format!("接口 {} 的 IP: {}", current_interface, ip));
                        if !ip.starts_with("127.") && ip.contains('.') {
                            Self::write_log(&format!("检测到物理网卡: {} IP: {}", current_interface, ip));
                            return Some(ip.to_string());
                        }
                    }
                    current_interface = String::new();
                }
            }
            Self::write_log("未找到有效的物理网卡 IP");
            None
        }
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("ip").args(["addr", "show"]).output().ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_interface = String::new();
            for line in stdout.lines() {
                if line.contains(": ") && !line.contains("tun") && !line.contains("lo") {
                    if let Some(name) = line.split(": ").nth(1) {
                        current_interface = name.split('@').next().unwrap_or(name).to_string();
                    }
                } else if !current_interface.is_empty() && line.trim().starts_with("inet ") {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        let ip = parts[1].split('/').next().unwrap_or("");
                        if !ip.starts_with("127.") && ip.contains('.') {
                            return Some(ip.to_string());
                        }
                    }
                    current_interface = String::new();
                }
            }
            None
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        { None }
    }

    pub async fn start(&self, config: TunConfig) -> Result<TunStatus, TunError> {
        let mut inner = self.inner.write().await;
        if inner.running { return Err(TunError::AlreadyRunning); }

        let mut config = config;
        Self::write_log("开始检测物理网卡 IP...");
        if config.physical_ip.is_empty() {
            Self::write_log("物理网卡 IP 为空，开始检测...");
            if let Some(physical_ip) = Self::get_physical_interface_ip() {
                Self::write_log(&format!("检测到物理网卡 IP: {}", physical_ip));
                config.physical_ip = physical_ip;
            } else {
                Self::write_log("无法检测物理网卡 IP，将使用默认路由");
            }
        }

        let mut tun_config = Configuration::default();
        tun_config.address(&config.tunnel_ip).netmask(&config.tunnel_netmask)
            .destination("10.0.0.1").mtu(config.mtu).up();

        #[cfg(target_os = "linux")] { tun_config.name(&config.device_name); }
        #[cfg(target_os = "windows")] { tun_config.name("Xnetify"); }

        let device = match tun::create_as_async(&tun_config) {
            Ok(d) => d,
            Err(e) => {
                let error_msg = e.to_string();
                return Err(TunError::DeviceCreationFailed(error_msg));
            }
        };

        let device_name = match device.tun_name() {
            Ok(name) if !name.is_empty() => name,
            _ => {
                #[cfg(target_os = "macos")]
                { Self::find_macos_tun_device(&config.tunnel_ip).unwrap_or_else(|| "unknown".to_string()) }
                #[cfg(not(target_os = "macos"))]
                { "tun0".to_string() }
            }
        };

        let mut final_config = config;
        final_config.device_name = device_name.clone();
        let device_arc = Arc::new(device);
        let device_for_processor = device_arc.clone();
        let config_clone = final_config.clone();
        let self_inner = self.inner.clone();
        let engine_clone = self.engine.clone();

        if let Err(e) = Self::setup_routes(&config_clone.tunnel_ip, &device_name) {
            log::warn!("⚠️ [TUN] 路由配置返回错误: {}", e);
        }

        let handle = tokio::spawn(async move {
            Self::packet_processor(device_for_processor, config_clone, self_inner, engine_clone).await;
        });

        inner.device = Some(device_arc);
        inner.config = Some(final_config);
        inner.running = true;
        inner.processor_handle = Some(handle);

        Ok(TunStatus { active: true, config: Some(inner.config.clone().unwrap()), bytes_in: 0, bytes_out: 0, error: None })
    }

    fn setup_routes(tunnel_ip: &str, device_name: &str) -> Result<(), TunError> {
        #[cfg(target_os = "macos")]
        {
            Self::write_log(&format!("开始配置 macOS 路由, 设备: {}, IP: {}", device_name, tunnel_ip));
            let current_route = std::process::Command::new("route").args(["-n", "get", "default"]).output();
            if let Ok(o) = &current_route {
                let stdout = String::from_utf8_lossy(&o.stdout);
                Self::write_log(&format!("当前默认路由:\n{}", stdout));
            }
            let _ = std::process::Command::new("route").args(["-n", "delete", "default"]).output();
            let _ = std::process::Command::new("route").args(["-n", "add", "default", "-interface", device_name]).output();
            Self::write_log("跳过 8.8.8.8 TUN 路由，DNS 将通过物理网卡发送");
            let _ = std::process::Command::new("route").args(["-n", "add", "-host", "127.0.0.1", "-interface", "lo0"]).output();
            let _ = std::process::Command::new("route").args(["-n", "delete", "-host", "192.168.1.1"]).output();
            let _ = std::process::Command::new("route").args(["-n", "add", "-host", "192.168.1.1", "192.168.1.1"]).output();
            let verify = std::process::Command::new("netstat").args(["-rn"]).output();
            if let Ok(o) = verify {
                let stdout = String::from_utf8_lossy(&o.stdout);
                Self::write_log(&format!("路由表:\n{}", stdout));
            }
            let _ = std::process::Command::new("route").args(["-n", "delete", "-inet6", "default"]).output();
            let _ = std::process::Command::new("route").args(["-n", "add", "-inet6", "default", "-interface", device_name]).output();
            let dns_result = std::process::Command::new("networksetup").args(["-setdnsservers", "Ethernet", "10.0.0.2"]).output();
            match dns_result {
                Ok(o) if o.status.success() => { Self::write_log("DNS 配置成功: 使用 10.0.0.2 (TUN)"); }
                _ => {}
            }
        }
        Ok(())
    }

    /// TUN 数据包处理器 — 核心数据流引擎
    /// 
    /// 数据流向:
    ///   客户端(10.0.0.2) → TUN设备recv() → 本处理器 → 根据协议分发:
    ///     DNS → handle_dns() → 拦截或转发到上游DNS
    ///     TCP → handle_tcp() → 通过SOCKS5代理转发到目标服务器
    ///
    /// 回环防护:
    ///   macOS 的 utun 设备有一个特性：当我们通过 device.send() 发送响应包时，
    ///   由于默认路由指向 TUN 接口，macOS 可能将这些响应包重新路由回 TUN 设备，
    ///   导致 device.recv() 再次读取到我们自己发送的包。如果不加以过滤，会形成
    ///   无限循环，导致 CPU 和网络流量飙升。
    ///
    ///   防护措施: 检查 IP 包的源地址。客户端发出的包源地址始终是 10.0.0.2，
    ///   而我们发送的响应包源地址是外部 IP（如 142.250.x.x）。因此只需过滤掉
    ///   源地址不是 10.0.0.2 的包即可打破回环。
    async fn packet_processor(
        device: Arc<AsyncDevice>,
        config: TunConfig,
        manager: Arc<RwLock<TunManagerInner>>,
        engine: Arc<crate::engine::FilterEngine>,
    ) {
        Self::write_log(&format!("TUN 数据包处理器已启动, DNS服务器: {}", config.dns_server));
        let mut buf = vec![0u8; 65535];
        let block_server_ip = [127u8, 0, 0, 1];
        let bytes_out = {
            let inner = manager.read().await;
            inner.bytes_out.clone()
        };

        loop {
            let n = match timeout(Duration::from_millis(100), device.recv(&mut buf)).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => { Self::write_log(&format!("读取数据包失败: {}", e)); continue; }
                Err(_) => continue,
            };
            if n < 20 { continue; }

            let packet = &buf[..n];
            if packet[0] >> 4 != 4 { continue; }

            let src_ip = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
            let dst_ip = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);
            let protocol = packet[9];

            // === 回环防护关键检查 ===
            // 客户端(10.0.0.2)发出的包源地址始终是 10.0.0.2
            // 我们发送的响应包源地址是外部IP，如果被路由回TUN，源地址就不是10.0.0.2
            // 通过此过滤可以打破 device.send() → macOS路由 → device.recv() 的回环
            if (protocol == 6 || protocol == 17) && packet.len() >= 40 {
                if src_ip != u32::from_be_bytes([10, 0, 0, 2]) {
                    Self::write_log(&format!("丢弃回环包: src={}.{}.{}.{}, dst={}.{}.{}.{}, proto={}, len={}",
                        src_ip >> 24, (src_ip >> 16) & 0xff, (src_ip >> 8) & 0xff, src_ip & 0xff,
                        dst_ip >> 24, (dst_ip >> 16) & 0xff, (dst_ip >> 8) & 0xff, dst_ip & 0xff,
                        protocol, n));
                    continue;
                }
            }

            {
                let mut inner = manager.write().await;
                if !inner.running { break; }
                inner.bytes_in += n as u64;
            }

            if Self::is_dns_packet(packet) {
                if let Err(e) = Self::handle_dns(packet, &config, device.clone(), &block_server_ip, &engine, &bytes_out).await {
                    Self::write_log(&format!("DNS 处理失败: {}", e));
                }
            } else if Self::is_tcp_packet(packet) {
                if let Err(e) = Self::handle_tcp(packet, device.clone(), &manager, &config.physical_ip, &bytes_out).await {
                    Self::write_log(&format!("TCP 处理失败: {}", e));
                }
            }
        }
    }

    async fn handle_dns(
        packet: &[u8], config: &TunConfig, device: Arc<AsyncDevice>,
        block_server_ip: &[u8; 4], engine: &Arc<crate::engine::FilterEngine>,
        bytes_out: &std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<(), TunError> {
        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        let dns_data_start = ip_header_len + 8;
        if let Some(dns_data) = packet.get(dns_data_start..) {
            if let Some(domain) = crate::sni_filter::DnsParser::extract_domain(dns_data) {
                let (allowed, reason) = engine.check_access(&domain, &format!("http://{}", domain));
                if !allowed {
                    if let Some(response) = Self::build_dns_blocked_response(packet, block_server_ip) {
                        bytes_out.fetch_add(response.len() as u64, std::sync::atomic::Ordering::Relaxed);
                        let _ = device.send(&response).await;
                    }
                } else {
                    if let Err(e) = Self::forward_dns_to_upstream(packet, &config.dns_server, &config.physical_ip, device.clone()).await {
                        Self::write_log(&format!("转发 DNS 失败: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    /// TCP 数据包处理器 — 实现 TUN ↔ SOCKS5 代理转发
    /// 
    /// 完整数据流:
    ///   客户端(10.0.0.2) → TUN recv() → handle_tcp()
    ///     ├── SYN: 立即回复 SYN-ACK(伪装成目标服务器) → 预注册连接占位 → 后台异步连接 SOCKS5
    ///     ├── ACK+数据: 查找连接状态 → 有tx则发送到SOCKS5 → 无tx则缓冲到pending_data
    ///     ├── ACK-only: 仅更新状态，不回复(防止ACK反馈循环)
    ///     └── FIN: 清理连接状态，发送FIN信号给SOCKS5端
    ///
    ///   SOCKS5 → TUN (后台spawn任务):
    ///     从SOCKS5读取数据 → 构建TCP响应包 → device.send()写回TUN → 客户端收到
    ///
    ///   回环防护要点:
    ///   1. packet_processor 层: 过滤源地址不是 10.0.0.2 的包(防止 device.send 回环)
    ///   2. ACK-only 处理: 不对纯ACK包回复ACK(防止ACK反馈循环)
    ///   3. 连接清理: SOCKS5端关闭时立即移除连接状态(防止后续ACK找不到连接)
    async fn handle_tcp(
        packet: &[u8], device: Arc<AsyncDevice>,
        manager: &Arc<RwLock<TunManagerInner>>, physical_ip: &str,
        bytes_out: &std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<(), TunError> {
        if packet.len() < 40 { return Ok(()); }

        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        let tcp_header_len = (((packet[ip_header_len + 12] >> 4) as usize) * 4).max(20);
        let tcp_flags = packet[ip_header_len + 13];

        let src_ip = format!("{}.{}.{}.{}", packet[12], packet[13], packet[14], packet[15]);
        let dst_ip = format!("{}.{}.{}.{}", packet[16], packet[17], packet[18], packet[19]);
        let src_port = u16::from_be_bytes([packet[ip_header_len], packet[ip_header_len + 1]]);
        let dst_port = u16::from_be_bytes([packet[ip_header_len + 2], packet[ip_header_len + 3]]);
        let seq = u32::from_be_bytes([packet[ip_header_len + 4], packet[ip_header_len + 5], packet[ip_header_len + 6], packet[ip_header_len + 7]]);
        let ack = u32::from_be_bytes([packet[ip_header_len + 8], packet[ip_header_len + 9], packet[ip_header_len + 10], packet[ip_header_len + 11]]);

        if dst_ip == "127.0.0.1" && (dst_port == 7890 || dst_port == 7891 || dst_port == 8888) { return Ok(()); }
        if dst_ip == "10.0.0.2" || dst_ip == "10.0.0.1" { return Ok(()); }

        let conn_key = format!("{}:{}->{}:{}", src_ip, src_port, dst_ip, dst_port);
        let is_syn = (tcp_flags & 0x02) != 0 && (tcp_flags & 0x10) == 0;
        let is_ack_only = (tcp_flags & 0x10) != 0 && (tcp_flags & 0x02) == 0 && (tcp_flags & 0x01) == 0;
        let is_fin = (tcp_flags & 0x01) != 0;
        let is_rst = (tcp_flags & 0x04) != 0;
        let has_data = packet.len() > ip_header_len + tcp_header_len;

        if is_rst { return Ok(()); }

        Self::write_log(&format!("TCP flags=0x{:02x} syn={} ack={} fin={} data={} {}:{}->{}:{}", 
            tcp_flags, is_syn, is_ack_only, is_fin, has_data, src_ip, src_port, dst_ip, dst_port));

        if is_syn {
            // === TCP 三路握手 — 第一阶段: SYN ===
            // 我们作为"中间人"拦截TCP连接:
            // 1. 立即向客户端发送 SYN-ACK(伪装成目标服务器)
            // 2. 后台异步连接 SOCKS5 代理(127.0.0.1:7891)
            // 3. SOCKS5 连接成功后注册 tx 通道
            //
            // 竞态条件防护:
            // 客户端收到 SYN-ACK 后几乎立刻发回 ACK(TUN环回<1ms),
            // 但 SOCKS5 握手可能需要 20-250ms。如果等 SOCKS5 完成才注册连接,
            // ACK 到达时会找不到连接。
            // 解决方案: 发送 SYN-ACK 后立即预注册连接占位(tx=None, pending_data=[]),
            // ACK 到达时能匹配到连接, 数据被缓冲到 pending_data,
            // 等 SOCKS5 完成后设置 tx 并刷新 pending_data。
            {
                let inner = manager.read().await;
                if inner.tcp_conns.contains_key(&conn_key) { return Ok(()); }
            }

            Self::write_log(&format!("TCP SYN: {}:{} -> {}:{}", src_ip, src_port, dst_ip, dst_port));

            let server_seq = chrono::Local::now().timestamp_subsec_nanos();
            let dst_ip_bytes_arr: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
            let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];

            let client_tcp_options = {
                let tcp_hdr_len = (((packet[ip_header_len + 12] >> 4) as usize) * 4).max(20);
                if tcp_hdr_len > 20 {
                    packet[ip_header_len + 20..ip_header_len + tcp_hdr_len].to_vec()
                } else {
                    vec![]
                }
            };

            // 立即发送 SYN-ACK 给客户端(伪装成目标服务器)
            let syn_ack = Self::build_tcp_packet_with_options(
                &dst_ip_bytes_arr, dst_port, &src_ip_bytes, src_port,
                server_seq, seq.wrapping_add(1), 0x12, &[], &client_tcp_options,
            );
            Self::write_log(&format!("立即发送 SYN-ACK: {}:{} <- {}:{}, seq={}, ack={}, pkt_len={}",
                src_ip, src_port, dst_ip, dst_port, server_seq, seq.wrapping_add(1), syn_ack.len()));
            bytes_out.fetch_add(syn_ack.len() as u64, std::sync::atomic::Ordering::Relaxed);
            if let Err(e) = device.send(&syn_ack).await {
                Self::write_log(&format!("发送 SYN-ACK 失败: {}", e));
                return Ok(());
            }

            // 预注册连接占位 — 关键! 让后续 ACK 包能找到连接
            let client_ack_store = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(seq.wrapping_add(1)));
            {
                let mut inner = manager.write().await;
                inner.tcp_conns.insert(conn_key.clone(), TcpConnState {
                    server_seq,
                    client_ack: client_ack_store.clone(),
                    tx_to_socks5: None,  // SOCKS5 连接完成后再设置
                    pending_data: Vec::new(),  // 缓冲在此期间到达的数据
                });
                Self::write_log(&format!("预注册连接占位: {}, 连接数: {}", conn_key, inner.tcp_conns.len()));
            }

            let device_clone = device.clone();
            let manager_clone = manager.clone();
            let conn_key_clone = conn_key.clone();
            let src_port_c = src_port;
            let dst_port_c = dst_port;
            let dst_ip_c = dst_ip.clone();
            let bytes_out_clone = bytes_out.clone();

            tokio::spawn(async move {
                let socks5_stream = match TcpStream::connect("127.0.0.1:7891").await {
                    Ok(s) => s,
                    Err(e) => {
                        Self::write_log(&format!("连接 SOCKS5 代理失败: {}", e));
                        Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                        {
                            let mut inner = manager_clone.write().await;
                            inner.tcp_conns.remove(&conn_key_clone);
                        }
                        return;
                    }
                };

                let mut stream = socks5_stream;
                if let Err(e) = stream.write_all(&[0x05, 0x01, 0x00]).await {
                    Self::write_log(&format!("SOCKS5 握手失败: {}", e));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }
                let mut auth_resp = [0u8; 2];
                if let Err(e) = timeout(Duration::from_secs(5), stream.read_exact(&mut auth_resp)).await {
                    Self::write_log(&format!("SOCKS5 认证响应超时: {}", e));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }
                if auth_resp[0] != 0x05 || auth_resp[1] != 0x00 {
                    Self::write_log(&format!("SOCKS5 认证失败: {:02x} {:02x}", auth_resp[0], auth_resp[1]));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }

                let dst_ip_bytes: Vec<u8> = dst_ip_c.split('.').filter_map(|s| s.parse().ok()).collect();
                let mut connect_req = vec![0x05, 0x01, 0x00, 0x01];
                connect_req.extend_from_slice(&dst_ip_bytes);
                connect_req.push((dst_port_c >> 8) as u8);
                connect_req.push((dst_port_c & 0xff) as u8);
                if let Err(_) = stream.write_all(&connect_req).await {
                    Self::write_log(&format!("发送 SOCKS5 连接请求失败"));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }
                let mut connect_resp = [0u8; 10];
                if let Err(_) = timeout(Duration::from_secs(5), stream.read_exact(&mut connect_resp)).await {
                    Self::write_log(&format!("SOCKS5 连接响应超时"));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }
                if connect_resp[0] != 0x05 || connect_resp[1] != 0x00 {
                    Self::write_log(&format!("SOCKS5 连接失败: {:02x} {:02x}", connect_resp[0], connect_resp[1]));
                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes[0], src_ip_bytes[1], src_ip_bytes[2], src_ip_bytes[3]), src_port_c, 0, seq.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                    {
                        let mut inner = manager_clone.write().await;
                        inner.tcp_conns.remove(&conn_key_clone);
                    }
                    return;
                }

                Self::write_log(&format!("SOCKS5 连接成功: {}:{}", dst_ip_c, dst_port_c));

                // 创建 TUN → SOCKS5 的数据通道
                // tx 用于主处理线程发送数据到 SOCKS5
                // rx 由下面的 TUN->SOCKS5 relay 任务消费
                let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
                let src_ip_bytes_c = src_ip_bytes;
                let dst_ip_bytes_c: [u8; 4] = [dst_ip_bytes[0], dst_ip_bytes[1], dst_ip_bytes[2], dst_ip_bytes[3]];

                let (mut read_half, mut write_half) = stream.into_split();
                let device_clone2 = device_clone.clone();

                // === 数据转发任务 1: SOCKS5 → TUN (物理网卡 → 代理 → 本任务 → TUN → 客户端) ===
                // 从 SOCKS5 代理读取目标服务器的响应数据,
                // 封装成 TCP 包通过 device.send() 写回 TUN 设备,
                // 客户端收到后以为是从目标服务器收到的数据。
                //
                // 注意: device.send() 写回的数据可能被 macOS 路由回 TUN (回环),
                // 但 packet_processor 层的源地址过滤会丢弃这些包(源地址不是 10.0.0.2)。
                let conn_key_for_read = conn_key_clone.clone();
                let manager_for_read = manager_clone.clone();
                let bytes_out_for_read = bytes_out_clone.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 65535];
                    let mut cur_seq = server_seq.wrapping_add(1);
                    loop {
                        match timeout(Duration::from_secs(30), read_half.read(&mut buf)).await {
                            Ok(Ok(0)) => {
                                // SOCKS5 端关闭连接(EOF) — 目标服务器或代理主动断开
                                Self::write_log(&format!("SOCKS5 连接关闭 (EOF): {}:{}", 
                                    dst_ip_bytes_c.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port_c));
                                let client_ack = client_ack_store.load(std::sync::atomic::Ordering::Relaxed);
                                // 发送 FIN+ACK 给客户端, 通知连接关闭
                                let fin = Self::build_tcp_packet(&dst_ip_bytes_c, dst_port_c, &src_ip_bytes_c, src_port_c, cur_seq, client_ack, 0x11, &[]);
                                bytes_out_for_read.fetch_add(fin.len() as u64, std::sync::atomic::Ordering::Relaxed);
                                let _ = device_clone2.send(&fin).await;
                                // 立即清理连接状态, 防止后续客户端 ACK 找不到连接
                                {
                                    let mut inner = manager_for_read.write().await;
                                    inner.tcp_conns.remove(&conn_key_for_read);
                                }
                                break;
                            }
                            Ok(Ok(n)) => {
                                // 从 SOCKS5 收到目标服务器的数据
                                let data = &buf[..n];
                                let client_ack = client_ack_store.load(std::sync::atomic::Ordering::Relaxed);
                                Self::write_log(&format!("从 SOCKS5 读取 {} bytes, seq={}, ack={}: {}:{}", 
                                    n, cur_seq, client_ack,
                                    dst_ip_bytes_c.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port_c));
                                // 封装成 TCP 数据包(带 PSH+ACK), 写回 TUN 设备
                                let resp = Self::build_tcp_packet(&dst_ip_bytes_c, dst_port_c, &src_ip_bytes_c, src_port_c, cur_seq, client_ack, 0x18, data);
                                bytes_out_for_read.fetch_add(resp.len() as u64, std::sync::atomic::Ordering::Relaxed);
                                if let Err(e) = device_clone2.send(&resp).await {
                                    Self::write_log(&format!("发送 TCP 响应失败: {}", e));
                                    {
                                        let mut inner = manager_for_read.write().await;
                                        inner.tcp_conns.remove(&conn_key_for_read);
                                    }
                                    break;
                                }
                                cur_seq = cur_seq.wrapping_add(n as u32);
                            }
                            Ok(Err(e)) => {
                                Self::write_log(&format!("读取 SOCKS5 数据失败: {}", e));
                                {
                                    let mut inner = manager_for_read.write().await;
                                    inner.tcp_conns.remove(&conn_key_for_read);
                                }
                                break;
                            }
                            Err(_) => { continue; }
                        }
                    }
                });

                // === 数据转发任务 2: TUN → SOCKS5 (客户端 → TUN → 本任务 → 代理 → 物理网卡) ===
                // 从 channel 接收主处理线程发来的数据(客户端发送的 TCP payload),
                // 写入 SOCKS5 代理, 由代理转发到目标服务器。
                // 当收到空数据(FIN信号)或写入失败时, 清理连接状态。
                let conn_key_for_relay = conn_key_clone.clone();
                let manager_for_relay = manager_clone.clone();
                tokio::spawn(async move {
                    while let Some(data) = rx.recv().await {
                        if data.is_empty() {
                            Self::write_log(&format!("TUN->SOCKS5 收到 FIN 信号，关闭: {}", conn_key_for_relay));
                            let mut inner = manager_for_relay.write().await;
                            inner.tcp_conns.remove(&conn_key_for_relay);
                            break;
                        }
                        if let Err(e) = write_half.write_all(&data).await {
                            Self::write_log(&format!("TUN->SOCKS5 写入失败: {}, {}", conn_key_for_relay, e));
                            let mut inner = manager_for_relay.write().await;
                            inner.tcp_conns.remove(&conn_key_for_relay);
                            break;
                        }
                    }
                    // Channel closed without empty data signal, still cleanup
                    let mut inner = manager_for_relay.write().await;
                    inner.tcp_conns.remove(&conn_key_for_relay);
                });

                // SOCKS5 握手完成, 注册 tx 通道并刷新缓冲数据
                // 在发送 SYN-ACK 后到 SOCKS5 连接成功期间,
                // 客户端可能已经发送了数据, 这些数据被缓冲在 pending_data 中,
                // 现在需要将这些数据发送到 SOCKS5 代理。
                {
                    let mut inner = manager_clone.write().await;
                    if let Some(state) = inner.tcp_conns.get_mut(&conn_key_clone) {
                        let pending = std::mem::take(&mut state.pending_data);
                        let tx = tx.clone();
                        state.tx_to_socks5 = Some(tx.clone());
                        // 必须在发送 pending 数据前释放锁, 避免死锁:
                        // 如果持有写锁的同时执行 .send().await, 而 recv 端需要获取写锁,
                        // 就会形成死锁。
                        drop(inner);
                        if !pending.is_empty() {
                            Self::write_log(&format!("刷新缓冲数据: {} 条, {}", pending.len(), conn_key_clone));
                        }
                        for data in pending {
                            if let Err(_) = tx.send(data).await {
                                Self::write_log(&format!("发送缓冲数据失败: {}", conn_key_clone));
                                break;
                            }
                        }
                    } else {
                        Self::write_log(&format!("刷新缓冲时未找到连接: {}", conn_key_clone));
                    }
                }
            });

            return Ok(());
        } else if is_fin {
            // === TCP 连接关闭 — 客户端发送 FIN ===
            // 客户端主动关闭连接, 清理状态并发送 FIN 信号给 SOCKS5 端
            let state = {
                let mut inner = manager.write().await;
                inner.tcp_conns.remove(&conn_key)
            };
            if let Some(state) = state {
                Self::write_log(&format!("TCP FIN: {}", conn_key));
                if let Some(tx) = &state.tx_to_socks5 {
                    let _ = tx.send(vec![]).await;  // 空数据 = FIN 信号
                }
                let dst_ip_bytes: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
                let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];
                let client_ack = state.client_ack.load(std::sync::atomic::Ordering::Relaxed);
                let fin_ack = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, state.server_seq, client_ack, 0x11, &[]);
                bytes_out.fetch_add(fin_ack.len() as u64, std::sync::atomic::Ordering::Relaxed);
                let _ = device.send(&fin_ack).await;
            }
        } else if has_data || is_ack_only {
            // === 数据处理 — 客户端发送了数据或纯 ACK ===
            if has_data {
                // 客户端发送了数据(有 payload)
                let data_start = ip_header_len + tcp_header_len;
                let data = packet[data_start..].to_vec();
                let new_ack = seq.wrapping_add((packet.len() - ip_header_len - tcp_header_len) as u32);
                let server_seq: u32;
                let dst_ip_bytes: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
                let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];
                let tx_result: Option<mpsc::Sender<Vec<u8>>>;
                {
                    let mut inner = manager.write().await;
                    if let Some(state) = inner.tcp_conns.get_mut(&conn_key) {
                        state.client_ack.store(new_ack, std::sync::atomic::Ordering::Relaxed);
                        server_seq = state.server_seq;
                        tx_result = state.tx_to_socks5.clone();
                        // tx 为 None 说明 SOCKS5 还没连接成功, 数据需要缓冲
                        if tx_result.is_none() {
                            Self::write_log(&format!("缓冲数据到 pending: {} bytes", data.len()));
                            state.pending_data.push(data);
                            drop(inner);
                            let ack_pkt = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, server_seq, new_ack, 0x10, &[]);
                            bytes_out.fetch_add(ack_pkt.len() as u64, std::sync::atomic::Ordering::Relaxed);
                            let _ = device.send(&ack_pkt).await;
                            return Ok(());
                        }
                        drop(inner);
                    } else {
                        // 连接不存在 — 可能已被 SOCKS5 端关闭
                        Self::write_log(&format!("未找到连接: {}", conn_key));
                        let rst = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, 0, seq.wrapping_add((packet.len() - ip_header_len - tcp_header_len) as u32), 0x14, &[]);
                        bytes_out.fetch_add(rst.len() as u64, std::sync::atomic::Ordering::Relaxed);
                        let _ = device.send(&rst).await;
                        return Ok(());
                    }
                }
                // tx 就绪, 发送数据到 SOCKS5
                let ack_pkt = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, server_seq, new_ack, 0x10, &[]);
                Self::write_log(&format!("发送 ACK (有数据): {}:{} <- {}:{}, seq={}, ack={}, data_len={}",
                    src_ip_bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), src_port,
                    dst_ip_bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port,
                    server_seq, new_ack, data.len()));
                bytes_out.fetch_add(ack_pkt.len() as u64, std::sync::atomic::Ordering::Relaxed);
                let _ = device.send(&ack_pkt).await;
                if let Err(_) = tx_result.unwrap().send(data).await {
                    Self::write_log(&format!("发送数据失败，移除连接: {}", conn_key));
                    let mut inner2 = manager.write().await;
                    inner2.tcp_conns.remove(&conn_key);
                }
            } else {
                // === 纯 ACK 包(无数据) — 关键: 不回复 ACK! ===
                // 如果我们对每个纯 ACK 包都回复一个 ACK, 会形成反馈循环:
                //   客户端 ACK → 我们回复 ACK → 客户端收到后发新 ACK → 无限循环
                // 这会导致每秒产生数万个 ACK 包, 流量飙升到 1.6-1.9 MB/s。
                // 正确做法: 只更新 client_ack 状态, 不发送任何回复。
                let new_ack = seq;
                {
                    let inner = manager.read().await;
                    if let Some(state) = inner.tcp_conns.get(&conn_key) {
                        state.client_ack.store(new_ack, std::sync::atomic::Ordering::Relaxed);
                    }
                    // 如果连接不存在, 静默丢弃 — 不需要发 RST,
                    // 因为连接可能刚被 SOCKS5 端关闭, 客户端的 ACK 是迟到的
                }
            }
        }

        Ok(())
    }

    /// TCP 直连处理器 — TUN → 目标服务器(不经过 SOCKS5)
    /// 
    /// 数据流向:
    ///   客户端(10.0.0.2) → TUN recv() → handle_tcp_v2()
    ///     ├── SYN: 立即回复 SYN-ACK → 预注册连接占位 → 后台异步直连目标服务器
    ///     ├── ACK+数据: 查找连接 → 有tx则发送到目标 → 无tx则缓冲到pending_data
    ///     ├── ACK-only: 仅更新状态，不回复(防止ACK反馈循环)
    ///     └── FIN: 清理连接状态，关闭目标连接
    ///
    ///   目标 → TUN (后台spawn任务):
    ///     从目标服务器读取数据 → 构建TCP响应包 → device.send()写回TUN → 客户端收到
    ///
    /// 优势: 无需 SOCKS5 代理中转, 延迟更低
    /// 注意: 直连流量需绑定物理网卡, 避免被路由回 TUN 设备
    async fn handle_tcp_v2(
        packet: &[u8], device: Arc<AsyncDevice>,
        manager: &Arc<RwLock<TunManagerInner>>, physical_ip: &str,
        bytes_out: &std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<(), TunError> {
        if packet.len() < 40 { return Ok(()); }

        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        let tcp_header_len = (((packet[ip_header_len + 12] >> 4) as usize) * 4).max(20);
        let tcp_flags = packet[ip_header_len + 13];

        let src_ip = format!("{}.{}.{}.{}", packet[12], packet[13], packet[14], packet[15]);
        let dst_ip = format!("{}.{}.{}.{}", packet[16], packet[17], packet[18], packet[19]);
        let src_port = u16::from_be_bytes([packet[ip_header_len], packet[ip_header_len + 1]]);
        let dst_port = u16::from_be_bytes([packet[ip_header_len + 2], packet[ip_header_len + 3]]);
        let seq = u32::from_be_bytes([packet[ip_header_len + 4], packet[ip_header_len + 5], packet[ip_header_len + 6], packet[ip_header_len + 7]]);
        let ack = u32::from_be_bytes([packet[ip_header_len + 8], packet[ip_header_len + 9], packet[ip_header_len + 10], packet[ip_header_len + 11]]);

        // 忽略发往本地服务的包
        if dst_ip == "127.0.0.1" && (dst_port == 7890 || dst_port == 7891 || dst_port == 8888) { return Ok(()); }
        if dst_ip == "10.0.0.2" || dst_ip == "10.0.0.1" { return Ok(()); }

        let conn_key = format!("{}:{}->{}:{}", src_ip, src_port, dst_ip, dst_port);
        let is_syn = (tcp_flags & 0x02) != 0 && (tcp_flags & 0x10) == 0;
        let is_ack_only = (tcp_flags & 0x10) != 0 && (tcp_flags & 0x02) == 0 && (tcp_flags & 0x01) == 0;
        let is_fin = (tcp_flags & 0x01) != 0;
        let is_rst = (tcp_flags & 0x04) != 0;
        let has_data = packet.len() > ip_header_len + tcp_header_len;

        if is_rst { return Ok(()); }

        Self::write_log(&format!("TCPv2 flags=0x{:02x} syn={} ack={} fin={} data={} {}:{}->{}:{}", 
            tcp_flags, is_syn, is_ack_only, is_fin, has_data, src_ip, src_port, dst_ip, dst_port));

        if is_syn {
            {
                let inner = manager.read().await;
                if inner.tcp_conns.contains_key(&conn_key) { return Ok(()); }
            }

            Self::write_log(&format!("TCPv2 SYN: {}:{} -> {}:{}", src_ip, src_port, dst_ip, dst_port));

            let server_seq = chrono::Local::now().timestamp_subsec_nanos();
            let dst_ip_bytes_arr: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
            let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];

            let client_tcp_options = {
                let tcp_hdr_len = (((packet[ip_header_len + 12] >> 4) as usize) * 4).max(20);
                if tcp_hdr_len > 20 {
                    packet[ip_header_len + 20..ip_header_len + tcp_hdr_len].to_vec()
                } else {
                    vec![]
                }
            };

            let syn_ack = Self::build_tcp_packet_with_options(
                &dst_ip_bytes_arr, dst_port, &src_ip_bytes, src_port,
                server_seq, seq.wrapping_add(1), 0x12, &[], &client_tcp_options,
            );
            Self::write_log(&format!("TCPv2 立即发送 SYN-ACK: {}:{} <- {}:{}, seq={}, ack={}, pkt_len={}",
                src_ip, src_port, dst_ip, dst_port, server_seq, seq.wrapping_add(1), syn_ack.len()));
            bytes_out.fetch_add(syn_ack.len() as u64, std::sync::atomic::Ordering::Relaxed);
            if let Err(e) = device.send(&syn_ack).await {
                Self::write_log(&format!("TCPv2 发送 SYN-ACK 失败: {}", e));
                return Ok(());
            }

            // 预注册连接占位
            let client_ack_store = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(seq.wrapping_add(1)));
            {
                let mut inner = manager.write().await;
                inner.tcp_conns.insert(conn_key.clone(), TcpConnState {
                    server_seq,
                    client_ack: client_ack_store.clone(),
                    tx_to_socks5: None,
                    pending_data: Vec::new(),
                });
                Self::write_log(&format!("TCPv2 预注册连接占位: {}, 连接数: {}", conn_key, inner.tcp_conns.len()));
            }

            let device_clone = device.clone();
            let manager_clone = manager.clone();
            let conn_key_clone = conn_key.clone();
            let src_port_c = src_port;
            let dst_port_c = dst_port;
            let dst_ip_c = dst_ip.clone();
            let bytes_out_clone = bytes_out.clone();
            let physical_ip_c = physical_ip.to_string();
            let dst_ip_bytes_v2: [u8; 4] = dst_ip_bytes_arr;
            let src_ip_bytes_v2: [u8; 4] = src_ip_bytes;
            let seq_v2 = seq;

            tokio::spawn(async move {
                let target_addr = format!("{}:{}", dst_ip_c, dst_port_c);
                Self::write_log(&format!("TCPv2 直连目标: {}", target_addr));

                // 添加临时主机路由, 让目标 IP 通过物理网关绕过 TUN
                // macOS 默认路由指向 TUN 后, 即使绑定物理网卡 IP 也无法直连
                let gateway = "192.168.1.1";
                let _ = std::process::Command::new("route")
                    .args(["-n", "add", "-host", &dst_ip_c, gateway])
                    .output();

                // 绑定到物理网卡, 避免流量被路由回 TUN
                let target_stream = if !physical_ip_c.is_empty() {
                    match physical_ip_c.parse::<std::net::IpAddr>() {
                        Ok(addr) => {
                            let socket = match tokio::net::TcpSocket::new_v4() {
                                Ok(s) => s,
                                Err(e) => {
                                    Self::write_log(&format!("TCPv2 创建 socket 失败: {}", e));
                                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                    { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                    return;
                                }
                            };
                            if let Err(e) = socket.bind(std::net::SocketAddr::new(addr, 0)) {
                                Self::write_log(&format!("TCPv2 绑定物理网卡失败: {}", e));
                                match timeout(Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
                                    Ok(Ok(s)) => s,
                                    Ok(Err(e)) => {
                                        Self::write_log(&format!("TCPv2 连接失败(回退): {}", e));
                                        Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                        { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                        return;
                                    }
                                    Err(_) => {
                                        Self::write_log(&format!("TCPv2 连接超时(回退)"));
                                        Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                        { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                        return;
                                    }
                                }
                            } else {
                                match target_addr.parse::<std::net::SocketAddr>() {
                                    Ok(addr) => {
                                        match timeout(Duration::from_secs(10), socket.connect(addr)).await {
                                            Ok(Ok(s)) => s,
                                            Ok(Err(e)) => {
                                                Self::write_log(&format!("TCPv2 连接失败: {}", e));
                                                Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                                { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                                return;
                                            }
                                            Err(_) => {
                                                Self::write_log(&format!("TCPv2 连接超时"));
                                                Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                                { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                                return;
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        match timeout(Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
                                            Ok(Ok(s)) => s,
                                            _ => {
                                                Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                                { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            match timeout(Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
                                Ok(Ok(s)) => s,
                                _ => {
                                    Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                                    { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                                    return;
                                }
                            }
                        }
                    }
                } else {
                    match timeout(Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
                        Ok(Ok(s)) => s,
                        _ => {
                            Self::send_rst(&dst_ip_c, dst_port_c, &format!("{}.{}.{}.{}", src_ip_bytes_v2[0], src_ip_bytes_v2[1], src_ip_bytes_v2[2], src_ip_bytes_v2[3]), src_port_c, 0, seq_v2.wrapping_add(1), device_clone.clone(), &bytes_out_clone).await;
                            { let mut inner = manager_clone.write().await; inner.tcp_conns.remove(&conn_key_clone); }
                            return;
                        }
                    }
                };

                Self::write_log(&format!("TCPv2 连接成功: {}:{}", dst_ip_c, dst_port_c));

                let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
                let src_ip_bytes_c = src_ip_bytes_v2;
                let dst_ip_bytes_c = dst_ip_bytes_v2;

                let (mut read_half, mut write_half) = target_stream.into_split();
                let device_clone2 = device_clone.clone();

                // === 数据转发任务 1: 目标服务器 → TUN ===
                let conn_key_for_read = conn_key_clone.clone();
                let manager_for_read = manager_clone.clone();
                let bytes_out_for_read = bytes_out_clone.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 65535];
                    let mut cur_seq = server_seq.wrapping_add(1);
                    loop {
                        match timeout(Duration::from_secs(30), read_half.read(&mut buf)).await {
                            Ok(Ok(0)) => {
                                Self::write_log(&format!("TCPv2 目标连接关闭 (EOF): {}:{}", 
                                    dst_ip_bytes_c.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port_c));
                                let client_ack = client_ack_store.load(std::sync::atomic::Ordering::Relaxed);
                                let fin = Self::build_tcp_packet(&dst_ip_bytes_c, dst_port_c, &src_ip_bytes_c, src_port_c, cur_seq, client_ack, 0x11, &[]);
                                bytes_out_for_read.fetch_add(fin.len() as u64, std::sync::atomic::Ordering::Relaxed);
                                let _ = device_clone2.send(&fin).await;
                                {
                                    let mut inner = manager_for_read.write().await;
                                    inner.tcp_conns.remove(&conn_key_for_read);
                                }
                                break;
                            }
                            Ok(Ok(n)) => {
                                let data = &buf[..n];
                                let client_ack = client_ack_store.load(std::sync::atomic::Ordering::Relaxed);
                                Self::write_log(&format!("TCPv2 从目标读取 {} bytes, seq={}, ack={}: {}:{}", 
                                    n, cur_seq, client_ack,
                                    dst_ip_bytes_c.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port_c));
                                let resp = Self::build_tcp_packet(&dst_ip_bytes_c, dst_port_c, &src_ip_bytes_c, src_port_c, cur_seq, client_ack, 0x18, data);
                                bytes_out_for_read.fetch_add(resp.len() as u64, std::sync::atomic::Ordering::Relaxed);
                                if let Err(e) = device_clone2.send(&resp).await {
                                    Self::write_log(&format!("TCPv2 发送 TCP 响应失败: {}", e));
                                    {
                                        let mut inner = manager_for_read.write().await;
                                        inner.tcp_conns.remove(&conn_key_for_read);
                                    }
                                    break;
                                }
                                cur_seq = cur_seq.wrapping_add(n as u32);
                            }
                            Ok(Err(e)) => {
                                Self::write_log(&format!("TCPv2 读取目标数据失败: {}", e));
                                {
                                    let mut inner = manager_for_read.write().await;
                                    inner.tcp_conns.remove(&conn_key_for_read);
                                }
                                break;
                            }
                            Err(_) => { continue; }
                        }
                    }
                });

                // === 数据转发任务 2: TUN → 目标服务器 ===
                let conn_key_for_relay = conn_key_clone.clone();
                let manager_for_relay = manager_clone.clone();
                tokio::spawn(async move {
                    while let Some(data) = rx.recv().await {
                        if data.is_empty() {
                            Self::write_log(&format!("TCPv2 收到 FIN 信号，关闭: {}", conn_key_for_relay));
                            let mut inner = manager_for_relay.write().await;
                            inner.tcp_conns.remove(&conn_key_for_relay);
                            break;
                        }
                        if let Err(e) = write_half.write_all(&data).await {
                            Self::write_log(&format!("TCPv2 写入目标失败: {}, {}", conn_key_for_relay, e));
                            let mut inner = manager_for_relay.write().await;
                            inner.tcp_conns.remove(&conn_key_for_relay);
                            break;
                        }
                    }
                    let mut inner = manager_for_relay.write().await;
                    inner.tcp_conns.remove(&conn_key_for_relay);
                });

                // 注册连接并刷新缓冲数据
                {
                    let mut inner = manager_clone.write().await;
                    if let Some(state) = inner.tcp_conns.get_mut(&conn_key_clone) {
                        let pending = std::mem::take(&mut state.pending_data);
                        let tx = tx.clone();
                        state.tx_to_socks5 = Some(tx.clone());
                        drop(inner);
                        if !pending.is_empty() {
                            Self::write_log(&format!("TCPv2 刷新缓冲数据: {} 条, {}", pending.len(), conn_key_clone));
                        }
                        for data in pending {
                            if let Err(_) = tx.send(data).await {
                                Self::write_log(&format!("TCPv2 发送缓冲数据失败: {}", conn_key_clone));
                                break;
                            }
                        }
                    } else {
                        Self::write_log(&format!("TCPv2 刷新缓冲时未找到连接: {}", conn_key_clone));
                    }
                }
            });

            return Ok(());
        } else if is_fin {
            let state = {
                let mut inner = manager.write().await;
                inner.tcp_conns.remove(&conn_key)
            };
            if let Some(state) = state {
                Self::write_log(&format!("TCPv2 FIN: {}", conn_key));
                if let Some(tx) = &state.tx_to_socks5 {
                    let _ = tx.send(vec![]).await;
                }
                let dst_ip_bytes: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
                let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];
                let client_ack = state.client_ack.load(std::sync::atomic::Ordering::Relaxed);
                let fin_ack = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, state.server_seq, client_ack, 0x11, &[]);
                bytes_out.fetch_add(fin_ack.len() as u64, std::sync::atomic::Ordering::Relaxed);
                let _ = device.send(&fin_ack).await;
            }
        } else if has_data || is_ack_only {
            if has_data {
                let data_start = ip_header_len + tcp_header_len;
                let data = packet[data_start..].to_vec();
                let new_ack = seq.wrapping_add((packet.len() - ip_header_len - tcp_header_len) as u32);
                let server_seq: u32;
                let dst_ip_bytes: [u8; 4] = [packet[16], packet[17], packet[18], packet[19]];
                let src_ip_bytes: [u8; 4] = [packet[12], packet[13], packet[14], packet[15]];
                let tx_result: Option<mpsc::Sender<Vec<u8>>>;
                {
                    let mut inner = manager.write().await;
                    if let Some(state) = inner.tcp_conns.get_mut(&conn_key) {
                        state.client_ack.store(new_ack, std::sync::atomic::Ordering::Relaxed);
                        server_seq = state.server_seq;
                        tx_result = state.tx_to_socks5.clone();
                        if tx_result.is_none() {
                            Self::write_log(&format!("TCPv2 缓冲数据到 pending: {} bytes", data.len()));
                            state.pending_data.push(data);
                            drop(inner);
                            let ack_pkt = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, server_seq, new_ack, 0x10, &[]);
                            bytes_out.fetch_add(ack_pkt.len() as u64, std::sync::atomic::Ordering::Relaxed);
                            let _ = device.send(&ack_pkt).await;
                            return Ok(());
                        }
                        drop(inner);
                    } else {
                        Self::write_log(&format!("TCPv2 未找到连接: {}", conn_key));
                        let rst = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, 0, seq.wrapping_add((packet.len() - ip_header_len - tcp_header_len) as u32), 0x14, &[]);
                        bytes_out.fetch_add(rst.len() as u64, std::sync::atomic::Ordering::Relaxed);
                        let _ = device.send(&rst).await;
                        return Ok(());
                    }
                }
                let ack_pkt = Self::build_tcp_packet(&dst_ip_bytes, dst_port, &src_ip_bytes, src_port, server_seq, new_ack, 0x10, &[]);
                Self::write_log(&format!("TCPv2 发送 ACK (有数据): {}:{} <- {}:{}, seq={}, ack={}, data_len={}",
                    src_ip_bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), src_port,
                    dst_ip_bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."), dst_port,
                    server_seq, new_ack, data.len()));
                bytes_out.fetch_add(ack_pkt.len() as u64, std::sync::atomic::Ordering::Relaxed);
                let _ = device.send(&ack_pkt).await;
                if let Err(_) = tx_result.unwrap().send(data).await {
                    Self::write_log(&format!("TCPv2 发送数据失败，移除连接: {}", conn_key));
                    let mut inner2 = manager.write().await;
                    inner2.tcp_conns.remove(&conn_key);
                }
            } else {
                let new_ack = seq;
                {
                    let inner = manager.read().await;
                    if let Some(state) = inner.tcp_conns.get(&conn_key) {
                        state.client_ack.store(new_ack, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(())
    }

    fn build_tcp_packet(
        src_ip: &[u8; 4], src_port: u16,
        dst_ip: &[u8; 4], dst_port: u16,
        seq: u32, ack_seq: u32,
        flags: u8,
        data: &[u8],
    ) -> Vec<u8> {
        Self::build_tcp_packet_with_options(src_ip, src_port, dst_ip, dst_port, seq, ack_seq, flags, data, &[])
    }

    fn build_tcp_packet_with_options(
        src_ip: &[u8; 4], src_port: u16,
        dst_ip: &[u8; 4], dst_port: u16,
        seq: u32, ack_seq: u32,
        flags: u8,
        data: &[u8],
        options: &[u8],
    ) -> Vec<u8> {
        let tcp_options_len = options.len();
        // Pad options to multiple of 4
        let padded_options_len = if tcp_options_len > 0 {
            ((tcp_options_len + 3) / 4) * 4
        } else {
            0
        };
        let tcp_header_len = 20 + padded_options_len;
        let data_offset = (tcp_header_len / 4) as u8;
        let mut pkt = Vec::with_capacity(20 + tcp_header_len + data.len());
        pkt.push(0x45); pkt.push(0x00);
        let total_len = (20 + tcp_header_len + data.len()) as u16;
        pkt.push((total_len >> 8) as u8); pkt.push((total_len & 0xff) as u8);
        pkt.push(0x00); pkt.push(0x00);
        pkt.push(0x40); pkt.push(0x00);
        pkt.push(64); pkt.push(6);
        pkt.push(0x00); pkt.push(0x00);
        pkt.extend_from_slice(src_ip); pkt.extend_from_slice(dst_ip);
        pkt.push((src_port >> 8) as u8); pkt.push((src_port & 0xff) as u8);
        pkt.push((dst_port >> 8) as u8); pkt.push((dst_port & 0xff) as u8);
        pkt.extend_from_slice(&seq.to_be_bytes());
        pkt.extend_from_slice(&ack_seq.to_be_bytes());
        pkt.push(data_offset << 4); pkt.push(flags);
        let window: u16 = 65535;
        pkt.push((window >> 8) as u8); pkt.push((window & 0xff) as u8);
        pkt.push(0x00); pkt.push(0x00);
        pkt.push(0x00); pkt.push(0x00);
        if !options.is_empty() {
            pkt.extend_from_slice(options);
            // Pad with zeros to multiple of 4
            for _ in 0..(padded_options_len - tcp_options_len) {
                pkt.push(0x00);
            }
        }
        pkt.extend_from_slice(data);
        let cs = Self::calc_ip_checksum(&pkt[..20]);
        pkt[10] = (cs >> 8) as u8; pkt[11] = (cs & 0xff) as u8;
        let tcp_cs = Self::calc_tcp_checksum(src_ip, dst_ip, 6, &pkt[20..]);
        pkt[36] = (tcp_cs >> 8) as u8; pkt[37] = (tcp_cs & 0xff) as u8;
        pkt
    }

    async fn send_rst(dst_ip: &str, dst_port: u16, src_ip: &str, src_port: u16, seq: u32, ack_seq: u32, device: Arc<AsyncDevice>, bytes_out: &std::sync::Arc<std::sync::atomic::AtomicU64>) {
        let d: [u8; 4] = dst_ip.split('.').filter_map(|s| s.parse().ok()).collect::<Vec<_>>().try_into().unwrap_or([0;4]);
        let s: [u8; 4] = src_ip.split('.').filter_map(|s| s.parse().ok()).collect::<Vec<_>>().try_into().unwrap_or([0;4]);
        let pkt = Self::build_tcp_packet(&d, dst_port, &s, src_port, seq, ack_seq, 0x14, &[]);
        bytes_out.fetch_add(pkt.len() as u64, std::sync::atomic::Ordering::Relaxed);
        let _ = device.send(&pkt).await;
    }

    fn calc_ip_checksum(header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let len = header.len();
        for i in (0..len - 1).step_by(2) {
            let word = ((header[i] as u32) << 8) | (header[i + 1] as u32);
            sum += word;
        }
        if len % 2 != 0 {
            sum += (header[len - 1] as u32) << 8;
        }
        while sum >> 16 != 0 { sum = (sum & 0xffff) + (sum >> 16); }
        !sum as u16
    }

    fn calc_tcp_checksum(src_ip: &[u8], dst_ip: &[u8], protocol: u8, tcp_data: &[u8]) -> u16 {
        let mut pseudo = Vec::with_capacity(12 + tcp_data.len());
        pseudo.extend_from_slice(src_ip); pseudo.extend_from_slice(dst_ip);
        pseudo.push(0); pseudo.push(protocol);
        pseudo.push((tcp_data.len() >> 8) as u8); pseudo.push((tcp_data.len() & 0xff) as u8);
        pseudo.extend_from_slice(tcp_data);
        Self::calc_ip_checksum(&pseudo)
    }

    fn is_tcp_packet(packet: &[u8]) -> bool {
        packet.len() >= 40 && packet[9] == 6
    }

    fn is_dns_packet(packet: &[u8]) -> bool {
        if packet.len() < 28 { return false; }
        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        if packet[9] != 17 { return false; }
        let dst_port = u16::from_be_bytes([packet[ip_header_len + 2], packet[ip_header_len + 3]]);
        dst_port == 53
    }

    fn build_dns_blocked_response(packet: &[u8], blocked_ip: &[u8; 4]) -> Option<Vec<u8>> {
        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        if packet.len() < ip_header_len + 8 + 12 { return None; }
        let mut resp = packet.to_vec();
        resp[12..16].copy_from_slice(&packet[16..20]);
        resp[16..20].copy_from_slice(&packet[12..16]);
        resp[ip_header_len..ip_header_len + 2].copy_from_slice(&packet[ip_header_len + 2..ip_header_len + 4]);
        resp[ip_header_len + 2..ip_header_len + 4].copy_from_slice(&packet[ip_header_len..ip_header_len + 2]);
        let dns_start = ip_header_len + 8;
        resp[dns_start + 2] = 0x81; resp[dns_start + 3] = 0x80;
        resp[dns_start + 6] = 0x00; resp[dns_start + 7] = 0x01;
        resp.push(0xC0); resp.push(0x0C);
        resp.push(0x00); resp.push(0x01); resp.push(0x00); resp.push(0x01);
        resp.push(0x00); resp.push(0x00); resp.push(0x00); resp.push(0x3C);
        resp.push(0x00); resp.push(0x04);
        resp.extend_from_slice(blocked_ip);
        let total_len = resp.len();
        resp[2] = (total_len >> 8) as u8; resp[3] = (total_len & 0xff) as u8;
        let udp_len = total_len - ip_header_len;
        resp[ip_header_len + 4] = (udp_len >> 8) as u8; resp[ip_header_len + 5] = (udp_len & 0xff) as u8;
        resp[10] = 0; resp[11] = 0;
        let cs = Self::calc_ip_checksum(&resp[..ip_header_len]);
        resp[10] = (cs >> 8) as u8; resp[11] = (cs & 0xff) as u8;
        let udp_cs = Self::calc_udp_checksum(&resp[16..20], &resp[12..16], 17, udp_len as u16, &resp[ip_header_len + 8..]);
        resp[ip_header_len + 6] = (udp_cs >> 8) as u8; resp[ip_header_len + 7] = (udp_cs & 0xff) as u8;
        Some(resp)
    }

    fn calc_udp_checksum(src_ip: &[u8], dst_ip: &[u8], protocol: u8, length: u16, data: &[u8]) -> u16 {
        let mut pseudo = Vec::with_capacity(12 + data.len());
        pseudo.extend_from_slice(src_ip); pseudo.extend_from_slice(dst_ip);
        pseudo.push(0); pseudo.push(protocol);
        pseudo.push((length >> 8) as u8); pseudo.push((length & 0xff) as u8);
        pseudo.extend_from_slice(data);
        Self::calc_ip_checksum(&pseudo)
    }

    async fn forward_dns_to_upstream(packet: &[u8], dns_server: &str, physical_ip: &str, device: Arc<AsyncDevice>) -> Result<(), TunError> {
        use tokio::net::UdpSocket;
        let ip_header_len = ((packet[0] & 0x0f) as usize) * 4;
        let dns_data_start = ip_header_len + 8;
        if let Some(dns_data) = packet.get(dns_data_start..) {
            let bind_addr = if physical_ip.is_empty() { "0.0.0.0:0".to_string() } else { format!("{}:0", physical_ip) };
            let socket = UdpSocket::bind(&bind_addr).await.map_err(|e| TunError::IoError(format!("绑定失败: {}", e)))?;
            socket.send_to(dns_data, format!("{}:53", dns_server)).await.map_err(|e| TunError::IoError(e.to_string()))?;
            let mut response_buf = vec![0u8; 512];
            let (len, _) = timeout(Duration::from_secs(5), socket.recv_from(&mut response_buf)).await
                .map_err(|_| TunError::UpstreamConnectionFailed("DNS 响应超时".to_string()))?
                .map_err(|e| TunError::IoError(e.to_string()))?;
            let mut full_response = Vec::new();
            full_response.extend_from_slice(&packet[..ip_header_len]);
            full_response[0] = 0x45;
            full_response[12..16].copy_from_slice(&packet[16..20]);
            full_response[16..20].copy_from_slice(&packet[12..16]);
            full_response.extend_from_slice(&packet[ip_header_len..ip_header_len + 4]);
            full_response[ip_header_len..ip_header_len + 2].copy_from_slice(&packet[ip_header_len + 2..ip_header_len + 4]);
            full_response[ip_header_len + 2..ip_header_len + 4].copy_from_slice(&packet[ip_header_len..ip_header_len + 2]);
            let udp_len = 8 + len;
            full_response.push((udp_len >> 8) as u8); full_response.push((udp_len & 0xff) as u8);
            full_response.push(0); full_response.push(0);
            full_response.extend_from_slice(&response_buf[..len]);
            let total_len = full_response.len();
            full_response[2] = (total_len >> 8) as u8; full_response[3] = (total_len & 0xff) as u8;
            full_response[10] = 0; full_response[11] = 0;
            let cs = Self::calc_ip_checksum(&full_response[..ip_header_len]);
            full_response[10] = (cs >> 8) as u8; full_response[11] = (cs & 0xff) as u8;
            let udp_cs = Self::calc_udp_checksum(&full_response[16..20], &full_response[12..16], 17, udp_len as u16, &full_response[ip_header_len + 8..]);
            full_response[ip_header_len + 6] = (udp_cs >> 8) as u8; full_response[ip_header_len + 7] = (udp_cs & 0xff) as u8;
            device.send(&full_response).await.map_err(|e| TunError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), TunError> {
        let mut inner = self.inner.write().await;
        if !inner.running { return Err(TunError::NotRunning); }
        Self::write_log("开始停止 TUN 设备...");
        Self::restore_network_config();
        if let Some(handle) = inner.processor_handle.take() { handle.abort(); }
        inner.device = None; inner.config = None; inner.running = false;
        inner.bytes_in = 0; inner.bytes_out.store(0, std::sync::atomic::Ordering::Relaxed); inner.tcp_conns.clear();
        Self::write_log("TUN 设备已停止，网络配置已恢复");
        Ok(())
    }

    fn restore_network_config() {
        #[cfg(target_os = "macos")]
        {
            Self::write_log("开始恢复 macOS 网络配置...");
            let _ = std::process::Command::new("route").args(["-n", "delete", "default"]).output();
            let _ = std::process::Command::new("route").args(["-n", "delete", "-inet6", "default"]).output();
            let _ = std::process::Command::new("route").args(["-n", "add", "default", "192.168.1.1"]).output();
            let _ = std::process::Command::new("networksetup").args(["-setdnsservers", "Ethernet", "Empty"]).output();
            Self::write_log("macOS 网络配置恢复完成");
        }
    }

    #[cfg(target_os = "macos")]
    fn find_macos_tun_device(expected_ip: &str) -> Option<String> {
        let output = std::process::Command::new("ifconfig").arg("-a").output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_device = String::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.ends_with(':') && line.contains("utun") {
                current_device = line.trim_end_matches(':').to_string();
            } else if current_device.starts_with("utun") && line.starts_with("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == expected_ip {
                    return Some(current_device);
                }
            }
        }
        None
    }

    pub async fn status(&self) -> TunStatus {
        let inner = self.inner.read().await;
        TunStatus { active: inner.running, config: inner.config.clone(), bytes_in: inner.bytes_in, bytes_out: inner.bytes_out.load(std::sync::atomic::Ordering::Relaxed), error: None }
    }

    pub async fn update_upstream(&self, host: String, port: u16) -> Result<(), TunError> {
        let mut inner = self.inner.write().await;
        if !inner.running { return Err(TunError::NotRunning); }
        if let Some(config) = &mut inner.config {
            config.upstream_host = host; config.upstream_port = port;
            Ok(())
        } else { Err(TunError::NotRunning) }
    }
}

impl Default for TunManager {
    fn default() -> Self {
        Self::new(Arc::new(crate::engine::FilterEngine::new(crate::models::FilterConfig::default())))
    }
}

#[tauri::command]
pub async fn tun_start(manager: tauri::State<'_, Arc<TunManager>>, host: String, port: u16) -> Result<TunResponse<TunStatus>, String> {
    let platform = crate::privilege::get_platform_name();
    #[cfg(target_os = "macos")] { log::info!("检测到 macOS 平台"); }
    if !crate::privilege::is_admin() {
        let instructions = crate::privilege::get_elevation_instructions();
        let hint = match platform {
            "macOS" => "sudo /Applications/xnetify.app/Contents/MacOS/xnetify",
            "Linux" => "sudo xnetify",
            "Windows" => "以管理员身份运行",
            _ => "请以管理员/root 权限运行应用",
        };
        return Ok(TunResponse::err(format!("需要管理员权限\n\n{}\n\n快速提示:\n{}", instructions, hint)));
    }
    let config = TunConfig {
        device_name: String::new(), tunnel_ip: "10.0.0.2".to_string(),
        tunnel_netmask: "255.255.255.0".to_string(), dns_server: "8.8.8.8".to_string(),
        mtu: 1500, upstream_host: host, upstream_port: port, physical_ip: String::new(),
    };
    match manager.start(config).await {
        Ok(status) => Ok(TunResponse::ok(status)),
        Err(e) => Ok(TunResponse::err(e.to_string())),
    }
}

#[tauri::command]
pub async fn tun_stop(manager: tauri::State<'_, Arc<TunManager>>) -> Result<TunResponse<String>, String> {
    match manager.stop().await {
        Ok(_) => Ok(TunResponse::ok("TUN 已停止".to_string())),
        Err(e) => Ok(TunResponse::err(e.to_string())),
    }
}

#[tauri::command]
pub async fn tun_get_status(manager: tauri::State<'_, Arc<TunManager>>) -> Result<TunResponse<TunStatus>, String> {
    Ok(TunResponse::ok(manager.status().await))
}

#[tauri::command]
pub async fn tun_update_upstream(manager: tauri::State<'_, Arc<TunManager>>, host: String, port: u16) -> Result<TunResponse<String>, String> {
    match manager.update_upstream(host, port).await {
        Ok(_) => Ok(TunResponse::ok("上游已更新".to_string())),
        Err(e) => Ok(TunResponse::err(e.to_string())),
    }
}
