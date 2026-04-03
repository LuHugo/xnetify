use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use crate::engine::FilterEngine;
use crate::sni_filter::SniExtractor;

const SOCKS_VERSION: u8 = 0x05;
const AUTH_NO_AUTH: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;

fn build_blocked_html(domain: &str, reason: Option<&str>) -> String {
    let reason_text = reason.unwrap_or("该网站可能包含不适合的内容");
    let html = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>访问受限 - 成长守护</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1e3a5f 0%, #0f172a 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 24px;
            padding: 48px;
            max-width: 480px;
            text-align: center;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
        }
        .icon {
            width: 80px;
            height: 80px;
            background: linear-gradient(135deg, #f59e0b, #f97316);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 24px;
        }
        .icon svg { width: 40px; height: 40px; fill: white; }
        h1 { color: #1e293b; font-size: 28px; font-weight: 700; margin-bottom: 12px; }
        .subtitle { color: #64748b; font-size: 16px; margin-bottom: 24px; }
        .info-box { background: #f1f5f9; border-radius: 12px; padding: 20px; margin-bottom: 24px; }
        .domain { color: #ef4444; font-weight: 600; font-size: 18px; word-break: break-all; }
        .reason { color: #64748b; font-size: 14px; margin-top: 8px; }
        .tips { background: #fef3c7; border-left: 4px solid #f59e0b; border-radius: 0 8px 8px 0; padding: 16px; text-align: left; }
        .tips h3 { color: #92400e; font-size: 14px; font-weight: 600; margin-bottom: 8px; }
        .tips ul { color: #78350f; font-size: 13px; padding-left: 20px; }
        .tips li { margin-bottom: 4px; }
        .footer { margin-top: 24px; color: #94a3b8; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">
            <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/></svg>
        </div>
        <h1>此网站暂时无法访问</h1>
        <p class="subtitle">成长守护为你过滤了不适宜的内容</p>
        <div class="info-box">
            <div class="domain">DOMAIN_PLACEHOLDER</div>
            <div class="reason">REASON_PLACEHOLDER</div>
        </div>
        <div class="tips">
            <h3>温馨提示</h3>
            <ul>
                <li>可以尝试访问首页或搜索其他内容</li>
                <li>如有疑问，请联系家长</li>
                <li>推荐访问「探索精彩」页面发现优质内容</li>
            </ul>
        </div>
        <div class="footer">成长守护 · 保护每一次探索</div>
    </div>
</body>
</html>"#;
    html.replace("DOMAIN_PLACEHOLDER", domain)
       .replace("REASON_PLACEHOLDER", reason_text)
}

fn build_blocked_response(domain: &str, reason: Option<&str>) -> Vec<u8> {
    let html = build_blocked_html(domain, reason);
    let html_bytes = html.as_bytes();
    let content_length = html_bytes.len();
    
    let mut response = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\
        \r\n",
        content_length
    ).into_bytes();
    
    response.extend_from_slice(html_bytes);
    response
}

fn build_redirect_response(html: &str) -> Vec<u8> {
    let html_bytes = html.as_bytes();
    let content_length = html_bytes.len();
    
    let mut response = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\
        Cache-Control: no-cache, no-store, must-revalidate\r\n\
        Pragma: no-cache\r\n\
        Expires: 0\r\n\
        \r\n",
        content_length
    ).into_bytes();
    
    response.extend_from_slice(html_bytes);
    response
}

fn build_blocked_connect_response(domain: &str, reason: Option<&str>) -> Vec<u8> {
    let html = build_blocked_html(domain, reason);
    let html_bytes = html.as_bytes();
    let content_length = html_bytes.len();
    
    format!(
        "HTTP/1.1 200 OK\r\n\
        Proxy-Agent: Xnetify/1.0\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\
        \r\n\
        {}",
        content_length,
        html
    ).into_bytes()
}

#[derive(Clone)]
pub struct ProxyConfig {
    pub local_port: u16,
    pub upstream_host: Option<String>,
    pub upstream_port: Option<u16>,
    pub direct_mode: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            local_port: 7890,
            upstream_host: None,
            upstream_port: None,
            direct_mode: true,
        }
    }
}

pub async fn start_http_proxy(
    local_port: u16,
    upstream_host: Option<&str>,
    filter_engine: Arc<FilterEngine>
) -> std::io::Result<()> {
    let local_addr = format!("127.0.0.1:{}", local_port);
    let listener = TcpListener::bind(&local_addr).await?;

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🌐 HTTP 代理服务启动");
    log::info!("📍 监听地址: {}", local_addr);
    if let Some(upstream) = upstream_host {
        log::info!("🔄 上游代理: {}", upstream);
    } else {
        log::info!("🔄 模式: 直连 (无上游代理)");
    }
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    loop {
        let (mut client, client_addr) = match listener.accept().await {
            Ok((client, addr)) => (client, addr),
            Err(e) => {
                log::error!("❌ 接受连接失败: {}", e);
                continue;
            }
        };

        let engine = filter_engine.clone();
        let upstream_str = upstream_host.map(|s| s.to_string());

        tokio::spawn(async move {
            handle_http_connection(&mut client, client_addr, upstream_str.as_deref(), &engine).await;
        });
    }
}

pub async fn start_socks5_proxy(
    local_port: u16, 
    upstream_host: Option<&str>,
    filter_engine: Arc<FilterEngine>,
    bind_ip: Option<String>
) -> std::io::Result<()> {
    let local_addr = format!("127.0.0.1:{}", local_port);
    let listener = TcpListener::bind(&local_addr).await?;

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("🔐 SOCKS5 代理服务启动");
    log::info!("📍 监听地址: {}", local_addr);
    if let Some(upstream) = upstream_host {
        log::info!("🔄 上游代理: {}", upstream);
    } else {
        log::info!("🔄 模式: 直连 (无上游代理)");
    }
    if let Some(ref ip) = bind_ip {
        log::info!("🌐 出口绑定: {}", ip);
    }
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    loop {
        let (socket, client_addr) = match listener.accept().await {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                log::error!("❌ 接受连接失败: {}", e);
                continue;
            }
        };
        
        let engine = filter_engine.clone();
        let upstream_str = upstream_host.map(|s| s.to_string());
        let bind_ip_clone = bind_ip.clone();

        tokio::spawn(async move {
            handle_socks5_connection(socket, client_addr, upstream_str.as_deref(), &engine, bind_ip_clone.as_deref()).await;
        });
    }
}

async fn handle_http_connection(
    client: &mut TcpStream,
    _client_addr: std::net::SocketAddr,
    upstream: Option<&str>,
    engine: &Arc<FilterEngine>
) {
    let mut buf = [0u8; 8192];
    let n = match timeout(Duration::from_secs(5), client.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(_)) => return,
        Err(_) => return,
    };

    if n == 0 {
        return;
    }

    let request_str = String::from_utf8_lossy(&buf[..n]);
    
    if request_str.starts_with("CONNECT ") {
        handle_http_connect(client, &buf[..n], upstream, engine).await;
    } else {
        handle_http_direct(client, &buf[..n], upstream, engine).await;
    }
}

async fn handle_http_connect(
    client: &mut TcpStream,
    buf: &[u8],
    upstream: Option<&str>,
    engine: &Arc<FilterEngine>
) {
    let request = String::from_utf8_lossy(buf);
    let host_port = request
        .trim_start_matches("CONNECT ")
        .split_whitespace()
        .next()
        .unwrap_or("");
    
    let (host, port): (String, u16) = if host_port.contains(':') {
        let parts: Vec<&str> = host_port.split(':').collect();
        (
            parts[0].to_string(),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(443)
        )
    } else {
        (host_port.to_string(), 443)
    };

    let url = format!("https://{}", host_port);
    let (allowed, reason) = engine.check_access(&host, &url);
    
    log::info!("📡 [PROXY-HTTPS] 域名: {} | URL: {} | 允许: {}", host, url, allowed);
    
    if !allowed {
        log::warn!("🚫 [PROXY-BLOCK] 域名: {} | 原因: {:?}", host, reason);
        let block_response = build_blocked_connect_response(&host, reason.as_deref());
        let _ = client.write_all(&block_response).await;
        let _ = client.shutdown().await;
        return;
    }

    match upstream {
        Some(upstream_addr) => {
            tunnel_through_upstream(client, &host, port, upstream_addr).await;
        }
        None => {
            tunnel_direct(client, &host, port).await;
        }
    }
}

async fn handle_http_direct(
    client: &mut TcpStream,
    buf: &[u8],
    _upstream: Option<&str>,
    engine: &Arc<FilterEngine>
) {
    let request = String::from_utf8_lossy(buf);
    
    let (host, port) = extract_host_and_port_from_request(&request);
    if host.is_empty() {
        return;
    }

    let url = format!("http://{}:{}", host, port);
    let (allowed, reason) = engine.check_access(&host, &url);
    
    log::info!("📡 [PROXY-HTTP] 域名: {} | 端口: {} | 允许: {}", host, port, allowed);
    
    if !allowed {
        log::warn!("🚫 [PROXY-BLOCK] 域名: {} | 原因: {:?}", host, reason);
        let block_html = build_blocked_html(&host, reason.as_deref());
        let block_response = build_redirect_response(&block_html);
        let _ = client.write_all(&block_response).await;
        let _ = client.shutdown().await;
        return;
    }

    let target_addr = format!("{}:{}", host, port);
    let mut target = match timeout(Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            log::error!("❌ [PROXY-HTTP] 连接目标 {} 失败: {}", target_addr, e);
            let resp = b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = client.write_all(resp).await;
            return;
        }
        Err(_) => {
            log::error!("❌ [PROXY-HTTP] 连接目标 {} 超时", target_addr);
            let resp = b"HTTP/1.1 504 Gateway Timeout\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = client.write_all(resp).await;
            return;
        }
    };

    if let Err(e) = target.write_all(buf).await {
        log::error!("❌ [PROXY-HTTP] 发送请求失败: {}", e);
        return;
    }

    let _ = tokio::io::copy_bidirectional(client, &mut target).await;
}

/// 从 HTTP 请求中提取主机名和端口
/// 优先从请求行 URL 中提取(如 GET http://host:port/path), 其次从 Host 头提取
/// 默认端口为 80
fn extract_host_and_port_from_request(request: &str) -> (String, u16) {
    // 尝试从请求行 URL 中提取 (GET http://host:port/path)
    if let Some(first_line) = request.lines().next() {
        if let Some(url_start) = first_line.find("://") {
            let after_scheme = &first_line[url_start + 3..];
            if let Some(space_pos) = after_scheme.find(|c: char| c.is_whitespace()) {
                let host_port = &after_scheme[..space_pos];
                // 移除路径部分
                if let Some(slash_pos) = host_port.find('/') {
                    let host_port_path = &host_port[..slash_pos];
                    if let Some(colon_pos) = host_port_path.find(':') {
                        let h = host_port_path[..colon_pos].to_string();
                        let p = host_port_path[colon_pos + 1..].parse::<u16>().unwrap_or(80);
                        return (h, p);
                    }
                } else {
                    let hp = host_port.to_string();
                    if let Some(colon_pos) = hp.find(':') {
                        let h = hp[..colon_pos].to_string();
                        let p = hp[colon_pos + 1..].parse::<u16>().unwrap_or(80);
                        return (h, p);
                    }
                    return (hp, 80);
                }
            }
        }
    }

    // 从 Host 头提取
    for line in request.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.starts_with("host:") {
            let host_port = line.split(':').nth(1).map(|s| s.trim()).unwrap_or("");
            if let Some(colon_pos) = host_port.find(':') {
                let h = host_port[..colon_pos].to_string();
                let p = host_port[colon_pos + 1..].parse::<u16>().unwrap_or(80);
                return (h, p);
            }
            return (host_port.to_string(), 80);
        }
    }
    (String::new(), 80)
}

async fn tunnel_through_upstream(
    client: &mut TcpStream,
    target_host: &str,
    target_port: u16,
    upstream_addr: &str
) {
    log::info!("🔌 [HTTP] 连接到上游: {}", upstream_addr);
    
    let mut upstream = match timeout(Duration::from_secs(5), TcpStream::connect(upstream_addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            log::error!("❌ [HTTP] 连接上游失败: {}", e);
            return;
        }
        Err(_) => {
            log::error!("❌ [HTTP] 连接上游超时");
            return;
        }
    };

    let connect_request = format!("CONNECT {}:{}\r\n\r\n", target_host, target_port);
    log::debug!("📤 [HTTP] 发送CONNECT请求到上游");
    
    if let Err(e) = upstream.write_all(connect_request.as_bytes()).await {
        log::error!("❌ [HTTP] 发送CONNECT失败: {}", e);
        return;
    }

    let mut resp_buf = [0u8; 1024];
    match timeout(Duration::from_secs(5), upstream.read(&mut resp_buf)).await {
        Ok(Ok(_)) => {
            log::info!("✅ [HTTP] 上游响应成功");
        }
        _ => {
            log::warn!("⚠️ [HTTP] 上游响应超时或为空");
        }
    }

    let success_resp = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    if let Err(e) = client.write_all(success_resp).await {
        log::error!("❌ [HTTP] 发送成功响应失败: {}", e);
        return;
    }

    log::info!("🔄 [HTTP] 开始双向转发");
    match tokio::io::copy_bidirectional(client, &mut upstream).await {
        Ok(_) => log::info!("✅ [HTTP] 转发完成"),
        Err(e) => log::error!("❌ [HTTP] 转发错误: {}", e),
    }
}

async fn tunnel_direct(
    client: &mut TcpStream,
    target_host: &str,
    target_port: u16
) {
    let target_addr = format!("{}:{}", target_host, target_port);
    
    let mut target = match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
        Ok(Ok(s)) => s,
        _ => return,
    };

    let success_resp = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    if let Err(_) = client.write_all(success_resp).await {
        return;
    }

    let _ = tokio::io::copy_bidirectional(client, &mut target).await;
}

async fn handle_socks5_connection(
    mut client: TcpStream,
    client_addr: std::net::SocketAddr,
    upstream: Option<&str>,
    engine: &Arc<FilterEngine>,
    bind_ip: Option<&str>
) {
    log::info!("🔌 [SOCKS5] 新连接来自: {}", client_addr);
    
    let mut buf = [0u8; 262];
    
    let n = match timeout(Duration::from_secs(5), client.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        _ => {
            log::warn!("⚠️ [SOCKS5] 读取认证请求超时");
            return;
        }
    };

    if n < 3 || buf[0] != SOCKS_VERSION {
        log::warn!("⚠️ [SOCKS5] 无效的版本: {}", buf[0]);
        return;
    }

    let methods = &buf[2..n];
    if !methods.contains(&AUTH_NO_AUTH) {
        let _ = client.write_all(&[SOCKS_VERSION, 0xFF]).await;
        return;
    }

    let _ = client.write_all(&[SOCKS_VERSION, AUTH_NO_AUTH]).await;
    log::info!("✅ [SOCKS5] 认证成功，等待连接请求...");

    let n = match timeout(Duration::from_secs(10), client.read(&mut buf)).await {
        Ok(Ok(n)) => {
            log::info!("📥 [SOCKS5] 收到连接请求: {} bytes", n);
            n
        }
        Ok(Err(e)) => {
            log::warn!("⚠️ [SOCKS5] 读取连接请求失败: {}", e);
            return;
        }
        Err(e) => {
            log::warn!("⚠️ [SOCKS5] 读取连接请求超时: {}", e);
            return;
        }
    };

    if n < 10 || buf[0] != SOCKS_VERSION || buf[1] != CMD_CONNECT {
        log::warn!("⚠️ [SOCKS5] 无效的连接请求");
        return;
    }

    let atyp = buf[3];
    let (target_host, target_port) = match atyp {
        ATYP_IPV4 => {
            if n < 10 { 
                return; 
            }
            let ip = format!("{}.{}.{}.{}", buf[4], buf[5], buf[6], buf[7]);
            let port = u16::from_be_bytes([buf[8], buf[9]]);
            (ip, port)
        }
        ATYP_DOMAIN => {
            if n < 7 { 
                return; 
            }
            let domain_len = buf[4] as usize;
            if n < 5 + domain_len + 2 { 
                return; 
            }
            let domain = String::from_utf8_lossy(&buf[5..5 + domain_len]).to_string();
            let port = u16::from_be_bytes([buf[5 + domain_len], buf[6 + domain_len]]);
            (domain, port)
        }
        _ => {
            log::warn!("⚠️ [SOCKS5] 不支持的地址类型: {}", atyp);
            return;
        }
    };

    log::info!("📤 [SOCKS5] 连接请求: {}:{}", target_host, target_port);

    let url = if target_port == 443 {
        format!("https://{}", target_host)
    } else {
        format!("http://{}", target_host)
    };

    let (allowed, reason) = engine.check_access(&target_host, &url);
    
    if !allowed {
        log::warn!("🚫 [Block] 域名: {} | 原因: {:?}", target_host, reason);
        send_socks5_reply(&mut client, 0x02).await;
        let _ = client.shutdown().await;
        return;
    }

    // SNI 兜底过滤: 当目标是 IP 地址(如 TUN 流量或客户端先做了DNS),
    // 域名规则无法匹配。对于 443 端口的 HTTPS 连接, 从 TLS ClientHello
    // 中提取 SNI(真实域名), 进行二次过滤。
    //
    // 使用 peek() 读取而不消费数据, 后续隧道函数正常读取 ClientHello。
    let effective_host = if target_host.parse::<std::net::IpAddr>().is_ok() && target_port == 443 {
        match peek_and_extract_sni(&client).await {
            Some(sni) => {
                let sni_url = format!("https://{}", sni);
                let (sni_allowed, sni_reason) = engine.check_access(&sni, &sni_url);
                if !sni_allowed {
                    log::warn!("🚫 [Block-SNI] 域名: {} | 原因: {:?}", sni, sni_reason);
                    send_socks5_reply(&mut client, 0x02).await;
                    let _ = client.shutdown().await;
                    return;
                }
                log::info!("🔍 [SNI] 提取域名: {} (原目标: {})", sni, target_host);
                sni
            }
            None => target_host.clone(),
        }
    } else {
        target_host.clone()
    };

    match upstream {
        Some(upstream_addr) => {
            socks5_tunnel_upstream(&mut client, effective_host, target_port, atyp, upstream_addr, bind_ip).await;
        }
        None => {
            socks5_tunnel_direct(&mut client, effective_host, target_port, atyp, bind_ip).await;
        }
    }
}

/// 使用 peek() 从客户端流中读取 TLS ClientHello 并提取 SNI 域名
/// peek() 不会消费数据, 后续隧道函数可以正常读取
async fn peek_and_extract_sni(client: &TcpStream) -> Option<String> {
    let mut buf = vec![0u8; 512];
    match timeout(Duration::from_secs(3), client.peek(&mut buf)).await {
        Ok(Ok(n)) if n > 5 => {
            SniExtractor::extract_sni(&buf[..n])
        }
        _ => None,
    }
}

async fn socks5_tunnel_upstream(
    client: &mut TcpStream,
    target_host: String,
    target_port: u16,
    atyp: u8,
    upstream_addr: &str,
    bind_ip: Option<&str>
) {
    // 动态获取物理网卡 IP 和网关
    let info = if bind_ip.is_none() {
        get_physical_interface_info()
    } else {
        bind_ip.map(|ip| PhysicalInterfaceInfo {
            ip: ip.to_string(),
            gateway: "192.168.1.1".to_string(),
        })
    };
    
    let effective_bind_ip = info.as_ref().map(|i| i.ip.clone());
    let gateway = info.as_ref().map(|i| i.gateway.clone());
    
    // 添加路由让上游代理流量绕过 TUN
    if let Some(ref gw) = gateway {
        if let Some((upstream_host, _)) = upstream_addr.split_once(':') {
            if upstream_host.parse::<std::net::IpAddr>().is_ok() {
                let _ = std::process::Command::new("route")
                    .args(["-n", "add", "-host", upstream_host, gw])
                    .output();
            }
        }
    }
    
    // 创建 socket 并绑定到物理网卡
    let mut upstream = if let Some(ref ip) = effective_bind_ip {
        match ip.parse::<std::net::IpAddr>() {
            Ok(addr) => {
                let socket = tokio::net::TcpSocket::new_v4().unwrap();
                let bind_addr = std::net::SocketAddr::new(addr, 0);
                if let Err(e) = socket.bind(bind_addr) {
                    log::warn!("⚠️ 绑定到 {} 失败: {}", ip, e);
                    match timeout(Duration::from_secs(5), TcpStream::connect(upstream_addr)).await {
                        Ok(Ok(s)) => s,
                        _ => {
                            send_socks5_reply(client, 0x04).await;
                            return;
                        }
                    }
                } else {
                    match upstream_addr.parse::<std::net::SocketAddr>() {
                        Ok(addr) => {
                            match timeout(Duration::from_secs(5), socket.connect(addr)).await {
                                Ok(Ok(s)) => s,
                                _ => {
                                    send_socks5_reply(client, 0x04).await;
                                    return;
                                }
                            }
                        }
                        Err(_) => {
                            match timeout(Duration::from_secs(5), TcpStream::connect(upstream_addr)).await {
                                Ok(Ok(s)) => s,
                                _ => {
                                    send_socks5_reply(client, 0x04).await;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                match timeout(Duration::from_secs(5), TcpStream::connect(upstream_addr)).await {
                    Ok(Ok(s)) => s,
                    _ => {
                        send_socks5_reply(client, 0x04).await;
                        return;
                    }
                }
            }
        }
    } else {
        match timeout(Duration::from_secs(5), TcpStream::connect(upstream_addr)).await {
            Ok(Ok(s)) => s,
            _ => {
                send_socks5_reply(client, 0x04).await;
                return;
            }
        }
    };

    let greeting = [SOCKS_VERSION, 0x01, AUTH_NO_AUTH];
    if upstream.write_all(&greeting).await.is_err() {
        send_socks5_reply(client, 0x01).await;
        return;
    }

    let mut auth_resp = [0u8; 2];
    if upstream.read_exact(&mut auth_resp).await.is_err() {
        send_socks5_reply(client, 0x01).await;
        return;
    }

    let mut request = vec![SOCKS_VERSION, CMD_CONNECT, 0x00];
    if atyp == ATYP_DOMAIN {
        let domain_bytes = target_host.as_bytes();
        request.push(ATYP_DOMAIN);
        request.push(domain_bytes.len() as u8);
        request.extend_from_slice(domain_bytes);
    } else {
        request.push(ATYP_IPV4);
        let ip_parts: Vec<&str> = target_host.split('.').collect();
        for part in ip_parts {
            request.push(part.parse().unwrap_or(0));
        }
    }
    request.extend_from_slice(&target_port.to_be_bytes());

    if upstream.write_all(&request).await.is_err() {
        send_socks5_reply(client, 0x01).await;
        return;
    }

    let resp_len = if atyp == ATYP_DOMAIN { 7 + target_host.len() } else { 10 };
    let mut response = vec![0u8; resp_len];
    
    if upstream.read_exact(&mut response).await.is_err() {
        send_socks5_reply(client, 0x01).await;
        return;
    }

    if response[0] != SOCKS_VERSION {
        send_socks5_reply(client, 0x01).await;
        return;
    }

    if client.write_all(&response).await.is_err() {
        return;
    }

    if response[1] != 0x00 {
        return;
    }

    let _ = tokio::io::copy_bidirectional(client, &mut upstream).await;
}

async fn socks5_tunnel_direct(
    client: &mut TcpStream,
    target_host: String,
    target_port: u16,
    _atyp: u8,
    bind_ip: Option<&str>
) {
    let target_addr = format!("{}:{}", target_host, target_port);
    
    // 动态获取物理网卡 IP 和网关
    let info = if bind_ip.is_none() {
        get_physical_interface_info()
    } else {
        bind_ip.map(|ip| PhysicalInterfaceInfo {
            ip: ip.to_string(),
            gateway: "192.168.1.1".to_string(),
        })
    };
    
    let effective_bind_ip = info.as_ref().map(|i| i.ip.clone());
    let gateway = info.as_ref().map(|i| i.gateway.clone());
    
    log::info!("🌐 [SOCKS5] 目标: {}, 物理IP: {:?}, 网关: {:?}", target_addr, effective_bind_ip, gateway);
    
    // 添加临时路由：让目标 IP 通过真实网关绕过 TUN
    if let Some(ref gw) = gateway {
        if target_host.parse::<std::net::IpAddr>().is_ok() {
            let route_output = std::process::Command::new("route")
                .args(["-n", "add", "-host", &target_host, gw])
                .output();
            
            match route_output {
                Ok(o) if o.status.success() => {
                    log::info!("📍 [SOCKS5] 添加路由: {} -> gateway {}", target_host, gw);
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    log::warn!("⚠️ [SOCKS5] 添加路由失败: {}", stderr);
                }
                Err(e) => {
                    log::warn!("⚠️ [SOCKS5] 添加路由命令失败: {}", e);
                }
            }
        }
    }
    
    // 创建 socket 并绑定到物理网卡
    let mut target = if let Some(ref ip) = effective_bind_ip {
        match ip.parse::<std::net::IpAddr>() {
            Ok(addr) => {
                let socket = tokio::net::TcpSocket::new_v4().unwrap();
                let bind_addr = std::net::SocketAddr::new(addr, 0);
                if let Err(e) = socket.bind(bind_addr) {
                    log::warn!("⚠️ 绑定到 {} 失败: {}", ip, e);
                    match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
                        Ok(Ok(s)) => s,
                        _ => {
                            send_socks5_reply(client, 0x04).await;
                            return;
                        }
                    }
                } else {
                    match target_addr.parse::<std::net::SocketAddr>() {
                        Ok(addr) => {
                            match timeout(Duration::from_secs(5), socket.connect(addr)).await {
                                Ok(Ok(s)) => s,
                                Ok(Err(e)) => {
                                    log::error!("❌ [SOCKS5] 连接失败: {}: {}", target_addr, e);
                                    send_socks5_reply(client, 0x04).await;
                                    return;
                                }
                                Err(e) => {
                                    log::error!("❌ [SOCKS5] 连接超时: {}: {}", target_addr, e);
                                    send_socks5_reply(client, 0x04).await;
                                    return;
                                }
                            }
                        }
                        Err(_) => {
                            match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
                                Ok(Ok(s)) => s,
                                _ => {
                                    send_socks5_reply(client, 0x04).await;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
                    Ok(Ok(s)) => s,
                    _ => {
                        send_socks5_reply(client, 0x04).await;
                        return;
                    }
                }
            }
        }
    } else {
        match timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await {
            Ok(Ok(s)) => s,
            _ => {
                send_socks5_reply(client, 0x04).await;
                return;
            }
        }
    };

    log::info!("✅ [SOCKS5] 连接成功: {} (出口: {:?})", target_addr, effective_bind_ip);

    let reply = build_socks5_reply(0x00, "0.0.0.0", 0);
    if client.write_all(&reply).await.is_err() {
        return;
    }

    let _ = tokio::io::copy_bidirectional(client, &mut target).await;
    
    // 清理临时路由
    if target_host.parse::<std::net::IpAddr>().is_ok() {
        let _ = std::process::Command::new("route")
            .args(["-n", "delete", "-host", &target_host])
            .output();
    }
}

fn get_physical_ip() -> Option<String> {
    get_physical_interface_info().map(|info| info.ip)
}

fn get_default_gateway() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("route").args(["-n", "get", "default"]).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gateway:") {
            return trimmed.trim_start_matches("gateway:").trim().to_string().into();
        }
    }
    
    None
}

struct PhysicalInterfaceInfo {
    ip: String,
    gateway: String,
}

fn get_physical_interface_info() -> Option<PhysicalInterfaceInfo> {
    use std::process::Command;
    
    let gateway = get_default_gateway().unwrap_or_else(|| "192.168.1.1".to_string());
    
    let output = Command::new("ifconfig").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let mut current_interface = String::new();
    
    for line in stdout.lines() {
        let is_interface_line = !line.starts_with('\t') 
            && !line.starts_with(' ')
            && line.contains(':')
            && line.len() > 1
            && line.chars().next().map_or(false, |c| c.is_alphanumeric());
        
        if is_interface_line && !line.contains("utun") && !line.contains("lo0") {
            if let Some(colon_pos) = line.find(':') {
                current_interface = line[..colon_pos].to_string();
            }
        } else if !current_interface.is_empty() && line.trim().starts_with("inet ") {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let ip = parts[1];
                if !ip.starts_with("127.") && ip.contains('.') {
                    return Some(PhysicalInterfaceInfo {
                        ip: ip.to_string(),
                        gateway: gateway.clone(),
                    });
                }
            }
            current_interface = String::new();
        }
    }
    
    None
}

async fn send_socks5_reply(client: &mut TcpStream, reply_code: u8) {
    let reply = [SOCKS_VERSION, reply_code, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let _ = client.write_all(&reply).await;
}

fn build_socks5_reply(reply_code: u8, _bind_addr: &str, bind_port: u16) -> Vec<u8> {
    let mut reply = vec![SOCKS_VERSION, reply_code, 0x00, 0x01];
    reply.extend_from_slice(&[0, 0, 0, 0]);
    reply.extend_from_slice(&bind_port.to_be_bytes());
    reply
}
