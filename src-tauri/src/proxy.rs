use std::net::Ipv4Addr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SOCKS_VERSION: u8 = 0x05;
const AUTH_NO_AUTH: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;

pub async fn start_http_proxy(local_port: u16, upstream_addr: &str) -> std::io::Result<()> {
    let local_addr = format!("127.0.0.1:{}", local_port);
    let listener = TcpListener::bind(&local_addr).await?;

    println!("🚀 ProxyFlow HTTP 代理已启动，监听: {}", local_addr);
    println!("🔗 流量将转发至: {}", upstream_addr);

    loop {
        let (mut client, client_addr) = listener.accept().await?;
        let upstream = upstream_addr.to_string();

        tokio::spawn(async move {
            println!("🆕 [HTTP] 新连接来自: {}", client_addr);

            match handle_http_proxy(&mut client, &upstream).await {
                Ok(_) => println!("✅ [HTTP] 连接关闭: {}", client_addr),
                Err(e) => eprintln!("❌ [HTTP] 连接错误: {}", e),
            }
        });
    }
}

pub async fn start_socks5_proxy(local_port: u16, upstream_addr: &str) -> std::io::Result<()> {
    let local_addr = format!("127.0.0.1:{}", local_port);
    let listener = TcpListener::bind(&local_addr).await?;

    println!("🚀 ProxyFlow SOCKS5 代理已启动，监听: {}", local_addr);
    println!("🔗 流量将转发至: {}", upstream_addr);

    loop {
        let (socket, client_addr) = listener.accept().await?;
        let upstream = upstream_addr.to_string();

        tokio::spawn(async move {
            println!("🆕 [SOCKS5] 新连接来自: {}", client_addr);

            match handle_socks5_connection(socket, &upstream).await {
                Ok(_) => println!("✅ [SOCKS5] 连接关闭: {}", client_addr),
                Err(e) => eprintln!("❌ [SOCKS5] 连接错误: {}", e),
            }
        });
    }
}

async fn handle_http_proxy(client: &mut TcpStream, upstream_addr: &str) -> std::io::Result<()> {
    let mut buf = [0u8; 4096];
    let n = client.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buf[..n]);

    if request.starts_with("CONNECT ") {
        let mut upstream = TcpStream::connect(upstream_addr).await?;

        upstream.write_all(&buf[..n]).await?;

        let resp_len = upstream.read(&mut buf).await?;
        if resp_len > 0 {
            client.write_all(&buf[..resp_len]).await?;
        }

        tokio::io::copy_bidirectional(client, &mut upstream).await?;
    } else {
        let mut upstream = TcpStream::connect(upstream_addr).await?;
        upstream.write_all(&buf[..n]).await?;
        tokio::io::copy_bidirectional(client, &mut upstream).await?;
    }

    Ok(())
}

async fn send_socks5_reply(client: &mut TcpStream, reply_code: u8) -> std::io::Result<()> {
    let reply = [SOCKS_VERSION, reply_code, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    client.write_all(&reply).await
}

async fn handle_socks5_connection(
    mut client: TcpStream,
    upstream_addr: &str,
) -> std::io::Result<()> {
    let mut buf = [0u8; 262];

    let n = client.read(&mut buf).await?;
    if n < 3 || buf[0] != SOCKS_VERSION {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid SOCKS5 greeting"));
    }

    let _nmethods = buf[1] as usize;
    let methods = &buf[2..n];

    if !methods.contains(&AUTH_NO_AUTH) {
        client.write_all(&[SOCKS_VERSION, 0xFF]).await?;
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "No acceptable auth method"));
    }

    client.write_all(&[SOCKS_VERSION, AUTH_NO_AUTH]).await?;

    let n = client.read(&mut buf).await?;
    if n < 10 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid SOCKS5 request"));
    }

    if buf[0] != SOCKS_VERSION || buf[1] != CMD_CONNECT {
        client.write_all(&[SOCKS_VERSION, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported command"));
    }

    let atyp = buf[3];
    let (target_host, target_port) = match atyp {
        ATYP_IPV4 => {
            if n < 10 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid IPv4 request"));
            }
            let ip = Ipv4Addr::new(buf[4], buf[5], buf[6], buf[7]);
            let port = u16::from_be_bytes([buf[8], buf[9]]);
            (ip.to_string(), port)
        }
        ATYP_DOMAIN => {
            if n < 7 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid domain request"));
            }
            let domain_len = buf[4] as usize;
            if n < 5 + domain_len + 2 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid domain request"));
            }
            let domain = String::from_utf8_lossy(&buf[5..5 + domain_len]).to_string();
            let port = u16::from_be_bytes([buf[5 + domain_len], buf[6 + domain_len]]);
            (domain, port)
        }
        _ => {
            client.write_all(&[SOCKS_VERSION, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported address type"));
        }
    };

    let target = format!("{}:{}", target_host, target_port);

    println!("🔌 [SOCKS5] 正在连接上游: {}", upstream_addr);
    let mut upstream = match TcpStream::connect(upstream_addr).await {
        Ok(stream) => {
            println!("✅ [SOCKS5] 已连接上游: {}", upstream_addr);
            stream
        }
        Err(e) => {
            eprintln!("❌ [SOCKS5] 无法连接到上游: {}", upstream_addr);
            send_socks5_reply(&mut client, 0x04).await.ok();
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    let greeting = [SOCKS_VERSION, 0x01, AUTH_NO_AUTH];
    println!("📤 [SOCKS5] 发送 greeting 到上游: {:?}", greeting);
    if let Err(e) = upstream.write_all(&greeting).await {
        eprintln!("❌ [SOCKS5] 发送 greeting 失败");
        send_socks5_reply(&mut client, 0x01).await.ok();
        return Err(e);
    }

    let mut auth_resp = [0u8; 2];
    match upstream.read_exact(&mut auth_resp).await {
        Ok(_) => {
            println!("📥 [SOCKS5] 收到上游 auth 响应: {:?}", auth_resp);
        }
        Err(e) => {
            eprintln!("❌ [SOCKS5] 读取上游 auth 响应失败: {}", e);
            send_socks5_reply(&mut client, 0x01).await.ok();
            return Err(e);
        }
    }

    if auth_resp[0] != SOCKS_VERSION {
        eprintln!("❌ [SOCKS5] 上游不支持 SOCKS5 协议");
        send_socks5_reply(&mut client, 0x01).await.ok();
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Upstream is not a SOCKS5 proxy"));
    }

    if auth_resp[1] != AUTH_NO_AUTH {
        eprintln!("❌ [SOCKS5] 上游认证失败");
        send_socks5_reply(&mut client, 0x01).await.ok();
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "SOCKS5 auth failed"));
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
            request.push(part.parse::<u8>().unwrap_or(0));
        }
    }
    request.extend_from_slice(&target_port.to_be_bytes());

    println!("📤 [SOCKS5] 发送请求到上游: {:?}", request);
    if let Err(e) = upstream.write_all(&request).await {
        eprintln!("❌ [SOCKS5] 发送请求失败");
        send_socks5_reply(&mut client, 0x01).await.ok();
        return Err(e);
    }

    let resp_len = match atyp {
        ATYP_IPV4 => 10,
        ATYP_DOMAIN => 7 + target_host.len(),
        _ => 10,
    };

    let mut response = vec![0u8; resp_len];
    match upstream.read_exact(&mut response).await {
        Ok(_) => {
            println!("📥 [SOCKS5] 收到上游响应: {:?}", response);
        }
        Err(e) => {
            eprintln!("❌ [SOCKS5] 读取上游响应失败: {}", e);
            send_socks5_reply(&mut client, 0x01).await.ok();
            return Err(e);
        }
    }

    if response[0] != SOCKS_VERSION {
        eprintln!("❌ [SOCKS5] 上游响应不是 SOCKS5 格式");
        let error_msg = String::from_utf8_lossy(&response).to_string();
        eprintln!("   上游响应: {}", error_msg.trim());
        send_socks5_reply(&mut client, 0x01).await.ok();
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Upstream not SOCKS5"));
    }

    let reply_code = response[1];
    println!("📤 [SOCKS5] 转发响应到客户端: {:?}", response);

    client.write_all(&response).await?;

    if reply_code != 0x00 {
        eprintln!("❌ [SOCKS5] 上游连接失败: {} -> reply_code={}", target, reply_code);
        return Ok(());
    }

    println!("✅ [SOCKS5] 连接成功: {}", target);

    match tokio::io::copy_bidirectional(&mut client, &mut upstream).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("❌ [SOCKS5] 数据传输错误: {}", e);
        }
    }

    Ok(())
}
