use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::engine::FilterEngine;

const DNS_PORT: u16 = 53;
const LOCAL_IP: &str = "127.0.0.1";

pub async fn start_dns_server(engine: Arc<FilterEngine>) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", DNS_PORT);
    let socket = UdpSocket::bind(&addr).await?;
    
    log::info!("🔒 DNS 拦截服务器启动: 127.0.0.1:{}", DNS_PORT);
    log::info!("💡 请将系统 DNS 设置为 127.0.0.1 来启用 DNS 拦截");
    
    let socket = Arc::new(socket);
    
    loop {
        let mut buf = [0u8; 512];
        match socket.recv_from(&mut buf).await {
            Ok((n, src)) => {
                let engine = engine.clone();
                let socket_clone = socket.clone();
                let packet = buf[..n].to_vec();
                tokio::spawn(async move {
                    if let Err(e) = handle_dns_query(&socket_clone, &packet, src, &engine).await {
                        log::error!("DNS 查询处理失败: {}", e);
                    }
                });
            }
            Err(e) => {
                log::error!("DNS 接收失败: {}", e);
            }
        }
    }
}

async fn handle_dns_query(
    socket: &UdpSocket,
    data: &[u8],
    src: std::net::SocketAddr,
    engine: &Arc<FilterEngine>
) -> std::io::Result<()> {
    if data.len() < 12 {
        return Ok(());
    }
    
    let _transaction_id = u16::from_be_bytes([data[0], data[1]]);
    let _query_type = u16::from_be_bytes([data[2], data[3]]);
    
    let domain = extract_domain_from_dns(data);
    
    let (allowed, reason) = engine.check_access(&domain, &format!("http://{}", domain));
    
    log::info!("📡 [DNS-SERVER] 域名: {} | 允许: {} | 原因: {:?}", domain, allowed, reason);
    
    let response = if allowed {
        build_dns_response(data, None)
    } else {
        log::warn!("🚫 [DNS-BLOCK] 域名: {} | 原因: {:?} | 解析到 127.0.0.1", domain, reason);
        build_dns_response(data, Some(LOCAL_IP))
    };
    
    let socket_ref = &socket;
    socket_ref.send_to(&response, src).await?;
    Ok(())
}

fn extract_domain_from_dns(data: &[u8]) -> String {
    let mut domain = String::new();
    let mut pos = 12;
    
    while pos < data.len() {
        let label_len = data[pos] as usize;
        if label_len == 0 {
            break;
        }
        pos += 1;
        
        if pos + label_len > data.len() {
            break;
        }
        
        if !domain.is_empty() {
            domain.push('.');
        }
        
        for i in 0..label_len {
            domain.push(data[pos + i] as char);
        }
        pos += label_len;
    }
    
    domain
}

fn build_dns_response(query: &[u8], blocked_ip: Option<&str>) -> Vec<u8> {
    if query.len() < 12 {
        return query.to_vec();
    }
    
    let qdcount = u16::from_be_bytes([query[4], query[5]]);
    
    let mut response = Vec::with_capacity(query.len() + 50);
    response.extend_from_slice(&query[0..2]);
    response.push(0x81);
    response.push(0x80);
    response.extend_from_slice(&[query[4], query[5]]);
    
    if blocked_ip.is_some() {
        response.extend_from_slice(&[0x00, 0x01]);
    } else {
        response.extend_from_slice(&[0x00, 0x00]);
    }
    
    response.extend_from_slice(&[0x00, 0x00]);
    response.extend_from_slice(&[0x00, 0x00]);
    
    let question_end = find_question_end(query);
    response.extend_from_slice(&query[12..question_end]);
    
    if let Some(ip_str) = blocked_ip {
        let ip_parts: Vec<u8> = ip_str.split('.').filter_map(|s| s.parse().ok()).collect();
        if ip_parts.len() == 4 {
            response.extend_from_slice(&[0xC0, 0x0C]);
            response.extend_from_slice(&[0x00, 0x01]);
            response.extend_from_slice(&[0x00, 0x01]);
            response.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]);
            response.extend_from_slice(&[0x00, 0x04]);
            response.extend_from_slice(&ip_parts);
        }
    }
    
    response
}

fn find_question_end(data: &[u8]) -> usize {
    let mut pos = 12;
    while pos < data.len() {
        if data[pos] == 0 {
            return pos + 5;
        }
        pos += 1;
    }
    data.len()
}

pub fn get_dns_port() -> u16 {
    DNS_PORT
}
