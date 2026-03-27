use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyApp {
    pub name: String,
    pub host: String,
    pub http_port: Option<u16>,
    pub socks_port: Option<u16>,
    pub all_ports: Vec<u16>,
    pub pid: Option<u32>,
    pub tested_http: Option<bool>,
    pub tested_socks: Option<bool>,
    pub tested: Option<bool>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCommand {
    pub shell_type: String,
    pub set_command: String,
    pub unset_command: String,
}

#[cfg(target_os = "macos")]
pub fn detect_proxy_ports_impl() -> Vec<ProxyApp> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps: Vec<ProxyApp> = vec![];

    let port_regex = Regex::new(r":(\d+)\s+\(LISTEN\)").unwrap();

    for line in stdout.lines() {
        let lower_line = line.to_lowercase();

        let proxy_keywords = [
            "lantern",
            "clash",
            "v2ray",
            "shadowsocks",
            "surge",
            "quantumult",
        ];
        let is_proxy = proxy_keywords.iter().any(|kw| lower_line.contains(kw));
        if !is_proxy {
            continue;
        }

        if let Some(caps) = port_regex.captures(line) {
            let port: u16 = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            if port == 0 {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            let process_name = parts.first().copied().unwrap_or("unknown");
            let pid: u32 = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);

            let app_name = detect_app_name(process_name);

            // Collect all ports
            if !apps.iter().any(|a| a.name == app_name) {
                apps.push(ProxyApp {
                    name: app_name.clone(),
                    host: "127.0.0.1".to_string(),
                    http_port: None,
                    socks_port: None,
                    all_ports: vec![port],
                    pid: Some(pid),
                    tested_http: None,
                    tested_socks: None,
                    tested: None,
                    latency_ms: None,
                });
            } else if let Some(existing) = apps.iter_mut().find(|a| a.name == app_name) {
                if !existing.all_ports.contains(&port) {
                    existing.all_ports.push(port);
                }
            }
        }
    }

    apps
}

#[cfg(target_os = "windows")]
pub fn detect_proxy_ports_impl() -> Vec<ProxyApp> {
    let output = Command::new("netstat").args(["-ano"]).output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps: Vec<ProxyApp> = vec![];

    let line_regex = Regex::new(r"TCP\s+[\d.]+:(\d+)\s+[\d.:]+\s+LISTENING\s+(\d+)").unwrap();

    let tasklist_output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output();
    let tasklist_str = tasklist_output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();

    for line in stdout.lines() {
        let lower_line = line.to_lowercase();

        if !lower_line.contains("lantern")
            && !lower_line.contains("clash")
            && !lower_line.contains("v2ray")
            && !lower_line.contains("shadowsocks")
        {
            continue;
        }

        if let Some(caps) = line_regex.captures(line) {
            let port: u16 = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let pid: u32 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);

            if port == 0 {
                continue;
            }

            let app_name = detect_app_name_from_tasklist(&tasklist_str, pid);

            // Collect all ports
            if !apps.iter().any(|a| a.name == app_name) {
                apps.push(ProxyApp {
                    name: app_name,
                    host: "127.0.0.1".to_string(),
                    http_port: None,
                    socks_port: None,
                    all_ports: vec![port],
                    pid: Some(pid),
                    tested_http: None,
                    tested_socks: None,
                    tested: None,
                    latency_ms: None,
                });
            } else if let Some(existing) = apps.iter_mut().find(|a| a.name == app_name) {
                if !existing.all_ports.contains(&port) {
                    existing.all_ports.push(port);
                }
            }
        }
    }

    apps
}

#[cfg(target_os = "linux")]
pub fn detect_proxy_ports_impl() -> Vec<ProxyApp> {
    let output = Command::new("ss").args(["-tlnp"]).output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            let fallback = Command::new("lsof")
                .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
                .output();
            match fallback {
                Ok(f) => f,
                Err(_) => return vec![],
            }
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps: Vec<ProxyApp> = vec![];

    let line_regex = Regex::new(r"\*:(\d+)\s+.*pid=(\d+)").unwrap();
    let simple_regex = Regex::new(r"\*:(\d+)").unwrap();

    let ps_output = Command::new("ps").args(["aux"]).output().ok();
    let ps_str = ps_output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();

    for line in stdout.lines() {
        let lower_line = line.to_lowercase();

        if !lower_line.contains("lantern")
            && !lower_line.contains("clash")
            && !lower_line.contains("v2ray")
            && !lower_line.contains("shadowsocks")
        {
            continue;
        }

        let port: u16 = if let Some(caps) = line_regex.captures(line) {
            caps.get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0)
        } else if let Some(caps) = simple_regex.captures(line) {
            caps.get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0)
        } else {
            continue;
        };

        if port == 0 {
            continue;
        }

        let app_name = detect_app_name_from_ps(&ps_str);

        // Collect all ports
        if !apps.iter().any(|a| a.name == app_name) {
            apps.push(ProxyApp {
                name: app_name,
                host: "127.0.0.1".to_string(),
                http_port: None,
                socks_port: None,
                all_ports: vec![port],
                pid: None,
                tested_http: None,
                tested_socks: None,
                tested: None,
                latency_ms: None,
            });
        } else if let Some(existing) = apps.iter_mut().find(|a| a.name == app_name) {
            if !existing.all_ports.contains(&port) {
                existing.all_ports.push(port);
            }
        }
    }

    apps
}

fn detect_app_name(process: &str) -> String {
    let lower = process.to_lowercase();
    if lower.contains("lantern") {
        "Lantern".to_string()
    } else if lower.contains("clash") {
        "Clash".to_string()
    } else if lower.contains("v2ray") || lower.contains("v2fly") {
        "V2Ray".to_string()
    } else if lower.contains("shadowsocks") || lower.contains("ss-local") {
        "Shadowsocks".to_string()
    } else if lower.contains("surge") {
        "Surge".to_string()
    } else if lower.contains("quantumult") {
        "Quantumult".to_string()
    } else {
        process.to_string()
    }
}

#[cfg(target_os = "windows")]
fn detect_app_name_from_tasklist(tasklist: &str, pid: u32) -> String {
    for line in tasklist.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            if let Ok(line_pid) = parts[1].trim().parse::<u32>() {
                if line_pid == pid {
                    if line.contains("lantern") {
                        return "Lantern".to_string();
                    } else if line.contains("clash") {
                        return "Clash".to_string();
                    } else if line.contains("v2ray") {
                        return "V2Ray".to_string();
                    }
                }
            }
        }
    }
    "Unknown".to_string()
}

#[cfg(target_os = "linux")]
fn detect_app_name_from_ps(ps: &str) -> String {
    for line in ps.lines() {
        if line.contains("lantern") {
            return "Lantern".to_string();
        } else if line.contains("clash") {
            return "Clash".to_string();
        } else if line.contains("v2ray") {
            return "V2Ray".to_string();
        }
    }
    "Unknown".to_string()
}

pub fn generate_proxy_command_impl(apps: Vec<ProxyApp>, shell_type: String) -> ProxyCommand {
    let first_app = apps.first();

    let http_port = first_app.and_then(|a| a.http_port);
    let socks_port = first_app.and_then(|a| a.socks_port);
    let localhost = "127.0.0.1";

    match shell_type.as_str() {
        "powershell" => {
            let cmd = if let Some(port) = http_port {
                format!(
                    r#"$env:HTTP_PROXY="http://{}:{}"; $env:HTTPS_PROXY="http://{}:{}"; echo "Proxy configured: HTTP {}:{}""#,
                    localhost, port, localhost, port, localhost, port
                )
            } else {
                String::from("echo No proxy port configured")
            };
            ProxyCommand {
                shell_type: "powershell".to_string(),
                set_command: cmd,
                unset_command: String::from(
                    r#"$env:HTTP_PROXY=$null; $env:HTTPS_PROXY=$null; echo "Proxy unset""#,
                ),
            }
        }
        "cmd" => {
            let cmd = if let Some(port) = http_port {
                format!("set HTTP_PROXY=http://{}:{} && set HTTPS_PROXY=http://{}:{} && echo Proxy configured: HTTP {}:{}", localhost, port, localhost, port, localhost, port)
            } else {
                String::from("echo No proxy port configured")
            };
            ProxyCommand {
                shell_type: "cmd".to_string(),
                set_command: cmd,
                unset_command: String::from(
                    "set HTTP_PROXY= && set HTTPS_PROXY= && echo Proxy unset",
                ),
            }
        }
        _ => {
            let set_cmd = if let (Some(http), Some(socks)) = (http_port, socks_port) {
                format!(
                    "export http_proxy=\"http://{}:{}\" && export https_proxy=\"http://{}:{}\" && export HTTP_PROXY=\"http://{}:{}\" && export HTTPS_PROXY=\"http://{}:{}\" && export ALL_PROXY=\"socks5://{}:{}\" && echo \"Proxy configured: HTTP {}:{}\"",
                    localhost, http, localhost, http,
                    localhost, http, localhost, http,
                    localhost, socks,
                    localhost, http
                )
            } else if let Some(port) = http_port {
                format!(
                    "export http_proxy=\"http://{}:{}\" && export https_proxy=\"http://{}:{}\" && export HTTP_PROXY=\"http://{}:{}\" && export HTTPS_PROXY=\"http://{}:{}\" && echo \"Proxy configured: HTTP {}:{}\"",
                    localhost, port, localhost, port,
                    localhost, port, localhost, port,
                    localhost, port
                )
            } else {
                String::from("echo No proxy port configured")
            };
            ProxyCommand {
                shell_type: "bash".to_string(),
                set_command: set_cmd,
                unset_command: String::from("unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY && echo \"Proxy unset\""),
            }
        }
    }
}

pub fn open_new_terminal_impl(command: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let escaped = command.replace('"', "`\"");
        let ps_command = format!(
            "powershell -NoExit -Command \"{}; Write-Host 'Press any key to exit...' -NoNewLine; $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')\"",
            escaped
        );
        std::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &ps_command])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(r#"tell application "Terminal" to do script "{}" "#, escaped);
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "xterm"];
        for term in terminals {
            let result = std::process::Command::new(term)
                .arg("-e")
                .arg("bash")
                .arg("-c")
                .arg(&command)
                .spawn();

            if result.is_ok() {
                return Ok(());
            }
        }
        return Err("No terminal emulator found".to_string());
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub async fn test_proxy_async(
    host: &str,
    port: u16,
    timeout_secs: u64,
    proxy_scheme: &str,
    test_urls: Vec<String>,
) -> ProxyTestResult {
    use reqwest::Client;
    use std::time::{Duration, Instant};

    let proxy_url = format!("{}://{}:{}", proxy_scheme, host, port);

    let proxy = match reqwest::Proxy::all(&proxy_url) {
        Ok(p) => p,
        Err(e) => {
            return ProxyTestResult {
                success: false,
                latency_ms: None,
                error: Some(format!("[{}] Invalid proxy URL: {}", port, e)),
            };
        }
    };

    let client = match Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProxyTestResult {
                success: false,
                latency_ms: None,
                error: Some(format!("[{}] Client build failed: {}", port, e)),
            };
        }
    };

    let start = Instant::now();
    let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

    for url in test_urls.iter() {
        match client.get(url).header("User-Agent", user_agent).send().await {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();
                if (200..400).contains(&status_code) {
                    return ProxyTestResult {
                        success: true,
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        error: None,
                    };
                }
                return ProxyTestResult {
                    success: false,
                    latency_ms: None,
                    error: Some(format!("[{}] HTTP {}", port, status_code)),
                };
            }
            Err(_) => {
                continue;
            }
        }
    }

    ProxyTestResult {
        success: false,
        latency_ms: None,
        error: Some(format!("[{}] All URLs failed", port)),
    }
}

pub async fn test_http_async(
    host: &str,
    port: u16,
    timeout_secs: u64,
    test_urls: Vec<String>,
) -> ProxyTestResult {
    test_proxy_async(host, port, timeout_secs, "http", test_urls).await
}

pub async fn test_socks_async(
    host: &str,
    port: u16,
    _timeout_secs: u64,
    _test_urls: Vec<String>,
) -> ProxyTestResult {
    use std::time::Instant;
    
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();
    
    match tokio::net::TcpStream::connect(&addr).await {
        Ok(mut stream) => {
            let greeting = [0x05, 0x01, 0x00];
            if let Err(_) = tokio::io::AsyncWriteExt::write_all(&mut stream, &greeting).await {
                return ProxyTestResult {
                    success: false,
                    latency_ms: None,
                    error: Some("Failed to send greeting".to_string()),
                };
            }
            
            let mut resp = [0u8; 2];
            match tokio::io::AsyncReadExt::read_exact(&mut stream, &mut resp).await {
                Ok(_) => {
                    if resp[0] == 0x05 && resp[1] == 0x00 {
                        return ProxyTestResult {
                            success: true,
                            latency_ms: Some(start.elapsed().as_millis() as u64),
                            error: None,
                        };
                    } else {
                        return ProxyTestResult {
                            success: false,
                            latency_ms: None,
                            error: Some(format!("Auth failed: {:?}", resp)),
                        };
                    }
                }
                Err(e) => {
                    return ProxyTestResult {
                        success: false,
                        latency_ms: None,
                        error: Some(format!("Read failed: {}", e)),
                    };
                }
            }
        }
        Err(e) => ProxyTestResult {
            success: false,
            latency_ms: None,
            error: Some(format!("Connect failed: {}", e)),
        },
    }
}

fn get_shell_config_path(shell_type: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match shell_type {
        "zsh" => home.join(".zshrc"),
        "bash" => {
            if home.join(".bashrc").exists() {
                home.join(".bashrc")
            } else {
                home.join(".bash_profile")
            }
        }
        _ => home.join(".zshrc"),
    }
}

fn get_proxy_env_lines(app: &ProxyApp, shell_type: &str) -> (String, String) {
    let localhost = "127.0.0.1";
    let http_port = app.http_port.unwrap_or(7890);
    let socks_port = app.socks_port.unwrap_or(7892);
    let app_name = &app.name;

    match shell_type {
        "zsh" | "bash" => {
            let set_lines = format!(
                r#"#PROXY FLOW START
# {} proxy config
export http_proxy="http://{}:{}"
export https_proxy="http://{}:{}"
export HTTP_PROXY="http://{}:{}"
export HTTPS_PROXY="http://{}:{}"
export ALL_PROXY="socks5://{}:{}"
#PROXY FLOW END"#,
                app_name,
                localhost, http_port,
                localhost, http_port,
                localhost, http_port,
                localhost, http_port,
                localhost, socks_port
            );
            let unset_lines = format!(
                r#"#PROXY FLOW START
# {} proxy unset
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY
#PROXY FLOW END"#,
                app_name
            );
            (set_lines, unset_lines)
        }
        "powershell" => {
            let set_lines = format!(
                r#"#PROXY FLOW START
# {} proxy config
$env:HTTP_PROXY="http://{}:{}"
$env:HTTPS_PROXY="http://{}:{}"
#PROXY FLOW END"#,
                app_name,
                localhost, http_port,
                localhost, http_port
            );
            let unset_lines = format!(
                r#"#PROXY FLOW START
# {} proxy unset
$env:HTTP_PROXY=$null
$env:HTTPS_PROXY=$null
#PROXY FLOW END"#,
                app_name
            );
            (set_lines, unset_lines)
        }
        _ => {
            let set_lines = format!(
                r#"#PROXY FLOW START
# {} proxy config
export http_proxy="http://{}:{}"
export https_proxy="http://{}:{}"
export HTTP_PROXY="http://{}:{}"
export HTTPS_PROXY="http://{}:{}"
export ALL_PROXY="socks5://{}:{}"
#PROXY FLOW END"#,
                app_name,
                localhost, http_port,
                localhost, http_port,
                localhost, http_port,
                localhost, http_port,
                localhost, socks_port
            );
            let unset_lines = String::new();
            (set_lines, unset_lines)
        }
    }
}

pub fn write_proxy_config_impl(app: ProxyApp, shell_type: String) -> Result<(), String> {
    let config_path = get_shell_config_path(&shell_type);
    let content = fs::read_to_string(&config_path).unwrap_or_default();

    let mut new_content = String::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.contains("#PROXY FLOW START") {
            in_block = true;
            continue;
        }
        if line.contains("#PROXY FLOW END") {
            in_block = false;
            continue;
        }
        if !in_block {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    let (set_lines, _) = get_proxy_env_lines(&app, &shell_type);
    new_content.push_str("\n");
    new_content.push_str(&set_lines);
    new_content.push('\n');

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&config_path)
        .map_err(|e| format!("Failed to open {}: {}", config_path.display(), e))?;

    file.write_all(new_content.as_bytes())
        .map_err(|e| format!("Failed to write to {}: {}", config_path.display(), e))?;

    Ok(())
}

pub fn remove_proxy_config_impl(shell_type: String) -> Result<(), String> {
    let config_path = get_shell_config_path(&shell_type);
    if !config_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&config_path).unwrap_or_default();
    let mut new_content = String::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.contains("#PROXY FLOW START") {
            in_block = true;
            continue;
        }
        if line.contains("#PROXY FLOW END") {
            in_block = false;
            continue;
        }
        if !in_block {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&config_path)
        .map_err(|e| format!("Failed to open {}: {}", config_path.display(), e))?;

    file.write_all(new_content.as_bytes())
        .map_err(|e| format!("Failed to write to {}: {}", config_path.display(), e))?;

    Ok(())
}

use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct NetworkProxySettings {
    pub web_proxy_enabled: bool,
    pub web_proxy_server: String,
    pub web_proxy_port: String,
    pub secure_web_proxy_enabled: bool,
    pub secure_web_proxy_server: String,
    pub secure_web_proxy_port: String,
    pub socks_proxy_enabled: bool,
    pub socks_proxy_server: String,
    pub socks_proxy_port: String,
}

#[derive(Default)]
pub struct ProxyStatus {
    pub enabled: Mutex<bool>,
    pub original_settings: Mutex<Vec<(String, NetworkProxySettings)>>,
}

impl ProxyStatus {
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
    }

    pub fn save_original_settings(&self, settings: Vec<(String, NetworkProxySettings)>) {
        *self.original_settings.lock().unwrap() = settings;
    }

    pub fn get_original_settings(&self) -> Vec<(String, NetworkProxySettings)> {
        self.original_settings.lock().unwrap().clone()
    }
}

#[cfg(target_os = "macos")]
pub fn set_system_proxy_impl(port: u16, state: &ProxyStatus) -> Result<String, String> {
    let services = get_all_active_network_services();
    
    if services.is_empty() {
        return Err("未找到活动网络服务".to_string());
    }

    let mut original_settings: Vec<(String, NetworkProxySettings)> = Vec::new();

    for service in &services {
        println!("📋 保存原始设置: {}", service);
        let settings = get_current_proxy_settings(service);
        original_settings.push((service.clone(), settings));
    }

    state.save_original_settings(original_settings);

    let mut success_count = 0;
    for service in &services {
        println!("🔧 设置代理: {}", service);

        let _ = Command::new("networksetup")
            .args(["-setwebproxy", service, "127.0.0.1", &port.to_string()])
            .output();
        
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxy", service, "127.0.0.1", &port.to_string()])
            .output();
        
        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxy", service, "127.0.0.1", &port.to_string()])
            .output();

        success_count += 1;
    }

    state.set_enabled(true);
    Ok(format!("系统代理已开启，已设置 {} 个网络", success_count))
}

#[cfg(target_os = "macos")]
pub fn unset_system_proxy_impl(state: &ProxyStatus) -> Result<String, String> {
    let original_settings = state.get_original_settings();

    if original_settings.is_empty() {
        let services = get_all_active_network_services();
        for service in &services {
            println!("🔧 关闭代理: {}", service);
            let _ = Command::new("networksetup")
                .args(["-setwebproxystate", service, "off"])
                .status();
            let _ = Command::new("networksetup")
                .args(["-setsecurewebproxystate", service, "off"])
                .status();
            let _ = Command::new("networksetup")
                .args(["-setsocksfirewallproxystate", service, "off"])
                .status();
        }
    } else {
        for (service, settings) in &original_settings {
            println!("🔄 恢复原始设置: {}", service);
            restore_proxy_settings(service, settings);
        }
    }

    state.set_enabled(false);
    Ok("系统代理已关闭".to_string())
}

#[cfg(target_os = "macos")]
fn get_all_active_network_services() -> Vec<String> {
    let output = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines()
        .filter(|line| !line.starts_with("*") && !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect()
}

#[cfg(target_os = "macos")]
fn get_current_proxy_settings(service: &str) -> NetworkProxySettings {
    let output = Command::new("networksetup")
        .args(["-getwebproxy", service])
        .output();

    let mut settings = NetworkProxySettings::default();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Enabled:") {
                settings.web_proxy_enabled = line.contains("Yes");
            } else if line.starts_with("Server:") {
                settings.web_proxy_server = line.replace("Server:", "").trim().to_string();
            } else if line.starts_with("Port:") {
                settings.web_proxy_port = line.replace("Port:", "").trim().to_string();
            }
        }
    }

    let output = Command::new("networksetup")
        .args(["-getsecurewebproxy", service])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Enabled:") {
                settings.secure_web_proxy_enabled = line.contains("Yes");
            } else if line.starts_with("Server:") {
                settings.secure_web_proxy_server = line.replace("Server:", "").trim().to_string();
            } else if line.starts_with("Port:") {
                settings.secure_web_proxy_port = line.replace("Port:", "").trim().to_string();
            }
        }
    }

    let output = Command::new("networksetup")
        .args(["-getsocksfirewallproxy", service])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Enabled:") {
                settings.socks_proxy_enabled = line.contains("Yes");
            } else if line.starts_with("Server:") {
                settings.socks_proxy_server = line.replace("Server:", "").trim().to_string();
            } else if line.starts_with("Port:") {
                settings.socks_proxy_port = line.replace("Port:", "").trim().to_string();
            }
        }
    }

    settings
}

#[cfg(target_os = "macos")]
fn restore_proxy_settings(service: &str, settings: &NetworkProxySettings) {
    if settings.web_proxy_enabled {
        let _ = Command::new("networksetup")
            .args(["-setwebproxy", service, &settings.web_proxy_server, &settings.web_proxy_port])
            .output();
    } else {
        let _ = Command::new("networksetup")
            .args(["-setwebproxystate", service, "off"])
            .output();
    }

    if settings.secure_web_proxy_enabled {
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxy", service, &settings.secure_web_proxy_server, &settings.secure_web_proxy_port])
            .output();
    } else {
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxystate", service, "off"])
            .output();
    }

    if settings.socks_proxy_enabled {
        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxy", service, &settings.socks_proxy_server, &settings.socks_proxy_port])
            .output();
    } else {
        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxystate", service, "off"])
            .output();
    }
}

#[cfg(target_os = "windows")]
pub fn set_system_proxy_impl(port: u16, state: &ProxyStatus) -> Result<String, String> {
    let localhost = "127.0.0.1";
    
    let http_url = format!("http://{}:{}", localhost, port);
    let https_url = format!("http://{}:{}", localhost, port);

    let _ = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "1", "/f",
        ])
        .status();

    let _ = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v", "ProxyServer", "/t", "REG_SZ", "/d", &format!("http={};https={}", http_url, https_url), "/f",
        ])
        .status();

    state.set_enabled(true);
    Ok(format!("系统代理已开启，指向端口 {}", port))
}

#[cfg(target_os = "windows")]
pub fn unset_system_proxy_impl(state: &ProxyStatus) -> Result<String, String> {
    let _ = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "0", "/f",
        ])
        .status();

    state.set_enabled(false);
    Ok("系统代理已关闭".to_string())
}

#[cfg(target_os = "linux")]
pub fn set_system_proxy_impl(port: u16, state: &ProxyStatus) -> Result<String, String> {
    let localhost = "127.0.0.1";
    let url = format!("http://{}:{}", localhost, port);

    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy.http", "host", localhost])
        .output()
        .map_err(|e| format!("设置代理失败: {}", e))?;

    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy.http", "port", &port.to_string()])
        .output()
        .map_err(|e| format!("设置代理端口失败: {}", e))?;

    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy.https", "host", localhost])
        .output()
        .map_err(|e| format!("设置 HTTPS 代理失败: {}", e))?;

    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy.https", "port", &port.to_string()])
        .output()
        .map_err(|e| format!("设置 HTTPS 代理端口失败: {}", e))?;

    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy", "mode", "manual"])
        .output()
        .map_err(|e| format!("启用代理模式失败: {}", e))?;

    state.set_enabled(true);
    Ok(format!("系统代理已开启，指向端口 {}", port))
}

#[cfg(target_os = "linux")]
pub fn unset_system_proxy_impl(state: &ProxyStatus) -> Result<String, String> {
    Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy", "mode", "none"])
        .output()
        .map_err(|e| format!("关闭代理失败: {}", e))?;

    state.set_enabled(false);
    Ok("系统代理已关闭".to_string())
}

pub fn is_proxy_enabled_impl(state: &ProxyStatus) -> bool {
    state.is_enabled()
}

#[derive(Clone, serde::Serialize)]
pub struct ProxyChangeInfo {
    pub service: String,
    pub changed_by: String,
}

#[cfg(target_os = "macos")]
pub fn check_proxy_changes_impl(port: u16) -> Vec<ProxyChangeInfo> {
    let services = get_all_active_network_services();
    let mut changes: Vec<ProxyChangeInfo> = Vec::new();
    let expected_port = port.to_string();

    for service in &services {
        let settings = get_current_proxy_settings(service);
        
        if settings.web_proxy_enabled && 
           !settings.web_proxy_server.is_empty() && 
           (settings.web_proxy_server != "127.0.0.1" || settings.web_proxy_port != expected_port) {
            changes.push(ProxyChangeInfo {
                service: service.clone(),
                changed_by: format!("{}:{}", settings.web_proxy_server, settings.web_proxy_port),
            });
        }
        
        if settings.secure_web_proxy_enabled && 
           !settings.secure_web_proxy_server.is_empty() && 
           (settings.secure_web_proxy_server != "127.0.0.1" || settings.secure_web_proxy_port != expected_port) {
            changes.push(ProxyChangeInfo {
                service: service.clone(),
                changed_by: format!("{}:{}", settings.secure_web_proxy_server, settings.secure_web_proxy_port),
            });
        }
        
        if settings.socks_proxy_enabled && 
           !settings.socks_proxy_server.is_empty() && 
           (settings.socks_proxy_server != "127.0.0.1" || settings.socks_proxy_port != expected_port) {
            changes.push(ProxyChangeInfo {
                service: service.clone(),
                changed_by: format!("{}:{}", settings.socks_proxy_server, settings.socks_proxy_port),
            });
        }
    }

    changes
}

#[cfg(target_os = "windows")]
pub fn check_proxy_changes_impl(_port: u16) -> Vec<ProxyChangeInfo> {
    vec![]
}

#[cfg(target_os = "linux")]
pub fn check_proxy_changes_impl(_port: u16) -> Vec<ProxyChangeInfo> {
    vec![]
}

