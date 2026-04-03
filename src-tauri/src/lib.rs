mod commands;
mod engine;
mod models;
mod nat;
mod privilege;
mod proxy;
mod sni_filter;
mod tun;
mod block_server;
mod dns_server;

use std::path::PathBuf;
use std::sync::Arc;
use engine::{FilterEngine, FilterEngineState, hash_pin};
use models::{FilterConfig, Mode, RecommendedSite, UrlRule, TimeRule};
use tun::TunManager;
use commands::{
    check_proxy_changes_impl, detect_proxy_ports_impl, generate_proxy_command_impl, is_proxy_enabled_impl,
    open_new_terminal_impl, remove_proxy_config_impl, set_system_proxy_impl,
    test_http_async, test_socks_async, unset_system_proxy_impl, write_proxy_config_impl, ProxyApp,
    ProxyCommand, ProxyChangeInfo, ProxyStatus, ProxyTestResult,
};
use tun::{
    tun_get_status, tun_start, tun_stop, tun_update_upstream,
};
use privilege::{
    check_admin, get_elevation_message, request_elevation,
};

#[tauri::command]
fn detect_proxy_ports() -> Vec<ProxyApp> {
    detect_proxy_ports_impl()
}

#[tauri::command]
fn generate_proxy_command(apps: Vec<ProxyApp>, shell_type: String) -> ProxyCommand {
    generate_proxy_command_impl(apps, shell_type)
}

#[tauri::command]
fn open_new_terminal(command: String) -> Result<(), String> {
    open_new_terminal_impl(command)
}

#[tauri::command]
fn write_proxy_config(app: ProxyApp, shell_type: String) -> Result<(), String> {
    write_proxy_config_impl(app, shell_type)
}

#[tauri::command]
fn remove_proxy_config(shell_type: String) -> Result<(), String> {
    remove_proxy_config_impl(shell_type)
}

#[tauri::command]
async fn set_system_proxy(port: u16, state: tauri::State<'_, ProxyStatus>) -> Result<String, String> {
    set_system_proxy_impl(port, &state)
}

#[tauri::command]
async fn unset_system_proxy(state: tauri::State<'_, ProxyStatus>) -> Result<String, String> {
    unset_system_proxy_impl(&state)
}

#[tauri::command]
fn is_proxy_enabled(state: tauri::State<'_, ProxyStatus>) -> bool {
    is_proxy_enabled_impl(state.inner())
}

#[tauri::command]
fn check_proxy_changes(port: u16) -> Vec<ProxyChangeInfo> {
    check_proxy_changes_impl(port)
}

#[tauri::command(rename_all = "camelCase")]
async fn test_http_port(
    host: String,
    port: u16,
    timeout_secs: Option<u64>,
    test_urls: Vec<String>,
) -> ProxyTestResult {
    let timeout = timeout_secs.unwrap_or(15);
    test_http_async(&host, port, timeout, test_urls).await
}

#[tauri::command(rename_all = "camelCase")]
async fn test_socks_port(
    host: String,
    port: u16,
    timeout_secs: Option<u64>,
    test_urls: Vec<String>,
) -> ProxyTestResult {
    let timeout = timeout_secs.unwrap_or(15);
    test_socks_async(&host, port, timeout, test_urls).await
}

#[tauri::command]
fn get_filter_state(engine: tauri::State<'_, Arc<FilterEngine>>) -> FilterEngineState {
    engine.get_state()
}

#[tauri::command]
fn set_mode(mode: Mode, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    engine.set_mode(mode);
    Ok(())
}

#[tauri::command]
fn check_url(url: String, engine: tauri::State<'_, Arc<FilterEngine>>) -> (bool, Option<String>) {
    let domain = extract_domain(&url);
    log::info!("🔍 [check_url] 手动测试 - URL: {}, 域名: {}", url, domain);
    
    if let Ok(config) = engine.get_config().read() {
        log::info!("🔍 [check_url] 引擎状态 - 模式: {:?}, 规则数量: {}", config.mode, config.url_rules.len());
        for (i, r) in config.url_rules.iter().enumerate() {
            log::info!("  规则{}: name={}, pattern={}, enabled={}", i+1, r.name, r.pattern, r.enabled);
        }
    }
    
    let result = engine.check_access(&domain, &url);
    log::info!("🔍 [check_url] 结果: 允许={}, 原因={:?}", result.0, result.1);
    result
}

#[tauri::command]
fn get_engine_state(engine: tauri::State<'_, Arc<FilterEngine>>) -> serde_json::Value {
    if let Ok(config) = engine.get_config().read() {
        let rules: Vec<serde_json::Value> = config.url_rules.iter().map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "pattern": r.pattern,
                "enabled": r.enabled,
                "rule_type": format!("{:?}", r.rule_type)
            })
        }).collect();
        
        serde_json::json!({
            "mode": format!("{:?}", config.mode),
            "url_rules": rules,
            "rule_count": config.url_rules.len()
        })
    } else {
        serde_json::json!({"error": "Failed to read config"})
    }
}

#[tauri::command]
fn add_url_rule(rule: UrlRule, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    log::info!("➕ [Command] 添加URL规则: name={}, pattern={}, enabled={}", rule.name, rule.pattern, rule.enabled);
    engine.add_url_rule(rule);
    
    if let Ok(config) = engine.get_config().read() {
        log::info!("📋 [Command] 当前规则数量: {}", config.url_rules.len());
        for (i, r) in config.url_rules.iter().enumerate() {
            log::info!("  规则{}: name={}, pattern={}, enabled={}", i+1, r.name, r.pattern, r.enabled);
        }
    }
    
    Ok(())
}



#[tauri::command]
fn remove_url_rule(rule_id: String, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    engine.remove_url_rule(&rule_id);
    // 清除系统 DNS 缓存, 确保删除规则后立即生效
    // macOS 缓存被拦截域名的 127.0.0.1 响应, 不清除会导致 TTL 内仍无法访问
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("killall")
            .args(["-HUP", "mDNSResponder"])
            .output();
        let _ = std::process::Command::new("dscacheutil")
            .args(["-flushcache"])
            .output();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl")
            .args(["restart", "systemd-resolved"])
            .output();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("ipconfig")
            .args(["/flushdns"])
            .output();
    }
    Ok(())
}

#[tauri::command]
fn set_time_rule(rule: TimeRule, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    if let Ok(mut config) = engine.get_config().write() {
        config.time_rule = rule;
    }
    Ok(())
}

#[tauri::command]
fn verify_pin(pin: String, engine: tauri::State<'_, Arc<FilterEngine>>) -> bool {
    let hash_store = engine.get_pin_hash();
    if let Ok(hash_opt) = hash_store.read() {
        if let Some(hash) = hash_opt.as_ref() {
            return engine.verify_pin(&pin, hash);
        }
    }
    true
}

#[tauri::command]
fn set_pin(pin: String, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    let hash = hash_pin(&pin);
    engine.set_pin_hash(hash);
    Ok(())
}

#[tauri::command]
fn get_recommended_sites(engine: tauri::State<'_, Arc<FilterEngine>>) -> Vec<RecommendedSite> {
    engine.get_recommended_sites()
}

#[tauri::command]
fn add_recommended_site(site: RecommendedSite, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    engine.add_recommended_site(site);
    Ok(())
}

#[tauri::command]
fn remove_recommended_site(site_id: String, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    engine.remove_recommended_site(&site_id);
    Ok(())
}

#[tauri::command]
fn update_recommended_site(site: RecommendedSite, engine: tauri::State<'_, Arc<FilterEngine>>) -> Result<(), String> {
    engine.update_recommended_site(site);
    Ok(())
}

fn extract_domain(url: &str) -> String {
    let without_scheme = url.trim_start_matches("https://").trim_start_matches("http://");
    without_scheme.split('/').next().unwrap_or(without_scheme).to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_upstream: Option<&str> = None;
    let socks_upstream: Option<&str> = None;
    let proxy_port = 7890;
    
    let mut filter_engine = FilterEngine::new(FilterConfig::default());
    
    // dev 和 release 使用不同的配置目录, 避免开发测试数据影响生产环境
    // dev:    ~/.xnetify-dev/filter_config.json
    // release: ~/.xnetify/filter_config.json
    let config_subdir = if cfg!(debug_assertions) {
        ".xnetify-dev"
    } else {
        ".xnetify"
    };
    
    if let Ok(app_data) = std::env::var("APPDATA") {
        // Windows: %APPDATA%\.xnetify\ 或 %APPDATA%\.xnetify-dev\
        let config_path = PathBuf::from(&app_data).join(config_subdir).join("filter_config.json");
        let _ = std::fs::create_dir_all(config_path.parent().unwrap());
        filter_engine = filter_engine.with_config_path(config_path.clone());
        filter_engine.load_from_disk();
        log::info!("🔧 配置路径: {:?}", config_path);
    } else if let Ok(home) = std::env::var("HOME") {
        // macOS/Linux: ~/.xnetify/ 或 ~/.xnetify-dev/
        let config_path = PathBuf::from(&home).join(config_subdir).join("filter_config.json");
        let _ = std::fs::create_dir_all(config_path.parent().unwrap());
        filter_engine = filter_engine.with_config_path(config_path.clone());
        filter_engine.load_from_disk();
        log::info!("🔧 配置路径: {:?}", config_path);
    }
    
    let filter_engine = Arc::new(filter_engine);
    let filter_engine_for_proxy = filter_engine.clone();
    
    let tun_manager = Arc::new(TunManager::new(filter_engine.clone()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ProxyStatus::default())
        .manage(tun_manager)
        .manage(filter_engine)
        .setup(move |_app| {
            let proxy_engine = filter_engine_for_proxy.clone();
            
            let engine_for_http = proxy_engine.clone();
            tauri::async_runtime::spawn(async move {
                proxy::start_http_proxy(proxy_port, http_upstream.as_deref(), engine_for_http)
                    .await
                    .unwrap_or_else(|e| log::error!("HTTP proxy error: {}", e));
            });

            let engine_for_socks = proxy_engine.clone();
            tauri::async_runtime::spawn(async move {
                proxy::start_socks5_proxy(7891, socks_upstream.as_deref(), engine_for_socks, None)
                    .await
                    .unwrap_or_else(|e| log::error!("SOCKS5 proxy error: {}", e));
            });

            let engine_for_block = proxy_engine.clone();
            tauri::async_runtime::spawn(async move {
                block_server::start_block_server(engine_for_block)
                    .await
                    .unwrap_or_else(|e| log::error!("Block server error: {}", e));
            });

            let engine_for_dns = proxy_engine.clone();
            tauri::async_runtime::spawn(async move {
                match dns_server::start_dns_server(engine_for_dns).await {
                    Ok(_) => {}
                    Err(e) => log::error!("DNS server error: {}", e),
                }
            });

            let _engine_for_proxy = proxy_engine;
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                if let Err(e) = set_system_proxy_impl(proxy_port, &ProxyStatus::default()) {
                    log::warn!("Failed to set system proxy on startup: {}", e);
                } else {
                    log::info!("System proxy enabled on startup");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_proxy_ports,
            generate_proxy_command,
            open_new_terminal,
            test_http_port,
            test_socks_port,
            write_proxy_config,
            remove_proxy_config,
            set_system_proxy,
            unset_system_proxy,
            is_proxy_enabled,
            check_proxy_changes,
            tun_start,
            tun_stop,
            tun_get_status,
            tun_update_upstream,
            check_admin,
            request_elevation,
            get_elevation_message,
            get_filter_state,
            get_engine_state,
            set_mode,
            check_url,
            add_url_rule,
            remove_url_rule,
            set_time_rule,
            verify_pin,
            set_pin,
            get_recommended_sites,
            add_recommended_site,
            remove_recommended_site,
            update_recommended_site,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
