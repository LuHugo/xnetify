use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::engine::FilterEngine;

const BLOCK_SERVER_PORT: u16 = 8888;

pub async fn start_block_server(engine: Arc<FilterEngine>) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", BLOCK_SERVER_PORT);
    let listener = TcpListener::bind(&addr).await?;
    
    log::info!("🔒 拦截页面服务器启动: http://127.0.0.1:{}", BLOCK_SERVER_PORT);
    
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_block_request(&mut socket, &engine).await {
                        log::error!("处理拦截请求失败: {}", e);
                    }
                });
            }
            Err(e) => {
                log::error!("接受连接失败: {}", e);
            }
        }
    }
}

async fn handle_block_request(
    socket: &mut TcpStream,
    _engine: &Arc<FilterEngine>
) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    let n = match socket.read(&mut buf).await {
        Ok(0) => return Ok(()),
        Ok(n) => n,
        Err(_) => return Ok(()),
    };
    
    let request = String::from_utf8_lossy(&buf[..n]);
    let domain = extract_domain_from_request(&request);
    
    if request.starts_with("CONNECT ") {
        let html = build_block_page_html(&domain);
        let response = format!(
            "HTTP/1.1 502 Bad Gateway\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            Proxy-Connection: close\r\n\
            \r\n\
            {}",
            html.len(),
            html
        );
        socket.write_all(response.as_bytes()).await?;
        let _ = socket.shutdown().await;
        return Ok(());
    }
    
    let html = build_block_page_html(&domain);
    let response = build_http_response(&html);
    
    socket.write_all(response.as_bytes()).await?;
    let _ = socket.shutdown().await;
    
    Ok(())
}

fn extract_domain_from_request(request: &str) -> String {
    for line in request.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.starts_with("host:") {
            if let Some(host) = line.split(':').nth(1) {
                let host = host.trim();
                if host.contains(':') {
                    return host.split(':').next().unwrap_or(host).to_string();
                }
                return host.to_string();
            }
        }
    }
    
    for line in request.lines() {
        if line.starts_with("GET ") || line.starts_with("POST ") {
            if let Some(url) = line.split_whitespace().nth(1) {
                if url.starts_with("http://") {
                    let without_scheme = url.trim_start_matches("http://");
                    return without_scheme.split('/').next().unwrap_or(without_scheme).to_string();
                }
            }
        }
    }
    
    "未知网站".to_string()
}

fn build_http_response(html: &str) -> String {
    let html_bytes = html.as_bytes();
    let content_length = html_bytes.len();
    
    format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\
        Cache-Control: no-cache\r\n\
        \r\n\
        {}",
        content_length,
        html
    )
}

fn build_block_page_html(domain: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>访问受限 - 成长守护</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1e3a5f 0%, #0f172a 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.98);
            border-radius: 24px;
            padding: 48px;
            max-width: 520px;
            width: 100%;
            text-align: center;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.4);
            animation: fadeIn 0.5s ease-out;
        }}
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(-20px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        .icon {{
            width: 100px;
            height: 100px;
            background: linear-gradient(135deg, #f59e0b, #f97316);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 28px;
            box-shadow: 0 10px 30px rgba(245, 158, 11, 0.3);
        }}
        .icon svg {{
            width: 50px;
            height: 50px;
            fill: white;
        }}
        h1 {{
            color: #1e293b;
            font-size: 32px;
            font-weight: 700;
            margin-bottom: 12px;
        }}
        .subtitle {{
            color: #64748b;
            font-size: 18px;
            margin-bottom: 32px;
            line-height: 1.6;
        }}
        .info-box {{
            background: #f1f5f9;
            border-radius: 16px;
            padding: 24px;
            margin-bottom: 28px;
        }}
        .domain {{
            color: #ef4444;
            font-weight: 700;
            font-size: 20px;
            word-break: break-all;
            margin-bottom: 8px;
        }}
        .reason {{
            color: #64748b;
            font-size: 15px;
        }}
        .tips {{
            background: #fef3c7;
            border-left: 4px solid #f59e0b;
            border-radius: 0 12px 12px 0;
            padding: 20px;
            text-align: left;
            margin-bottom: 24px;
        }}
        .tips h3 {{
            color: #92400e;
            font-size: 15px;
            font-weight: 600;
            margin-bottom: 12px;
            display: flex;
            align-items: center;
            gap: 8px;
        }}
        .tips ul {{
            color: #78350f;
            font-size: 14px;
            padding-left: 0;
            list-style: none;
        }}
        .tips li {{
            margin-bottom: 8px;
            padding-left: 24px;
            position: relative;
        }}
        .tips li:before {{
            content: '✓';
            position: absolute;
            left: 0;
            color: #f59e0b;
            font-weight: bold;
        }}
        .footer {{
            margin-top: 28px;
            color: #94a3b8;
            font-size: 13px;
        }}
        .back-link {{
            display: inline-block;
            margin-top: 20px;
            padding: 12px 28px;
            background: linear-gradient(135deg, #10b981, #059669);
            color: white;
            text-decoration: none;
            border-radius: 50px;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        .back-link:hover {{
            transform: translateY(-2px);
            box-shadow: 0 8px 20px rgba(16, 185, 129, 0.3);
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">
            <svg viewBox="0 0 24 24"><path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm0 10.99h7c-.53 4.12-3.28 7.79-7 8.94V12H5V6.3l7-3.11v8.8z"/></svg>
        </div>
        <h1>此网站暂时无法访问</h1>
        <p class="subtitle">成长守护为你过滤了不适宜的内容<br>帮助你安全、健康地探索互联网世界</p>
        
        <div class="info-box">
            <div class="domain">{domain}</div>
            <div class="reason">该网站被成长守护拦截</div>
        </div>
        
        <div class="tips">
            <h3>💡 温馨提示</h3>
            <ul>
                <li>可以尝试访问首页或搜索其他有益的内容</li>
                <li>发现有价值的在线学习资源</li>
                <li>如有需要，请联系家长协助</li>
                <li>推荐访问「探索精彩」页面发现优质内容</li>
            </ul>
        </div>
        
        <a href="javascript:history.back()" class="back-link">← 返回上一页</a>
        
        <div class="footer">
            成长守护 · 保护每一次探索
        </div>
    </div>
</body>
</html>"#, domain = domain)
}

pub fn get_block_server_port() -> u16 {
    BLOCK_SERVER_PORT
}

pub fn get_block_server_url() -> String {
    format!("http://127.0.0.1:{}", BLOCK_SERVER_PORT)
}
