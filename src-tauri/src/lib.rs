mod commands;
mod proxy;

use commands::{
    check_proxy_changes_impl, detect_proxy_ports_impl, generate_proxy_command_impl, is_proxy_enabled_impl,
    open_new_terminal_impl, remove_proxy_config_impl, set_system_proxy_impl,
    test_http_async, test_socks_async, unset_system_proxy_impl, write_proxy_config_impl, ProxyApp,
    ProxyCommand, ProxyChangeInfo, ProxyStatus, ProxyTestResult,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_upstream = "127.0.0.1:49836";
    let socks_upstream = "127.0.0.1:51068";

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ProxyStatus::default())
        .setup(move |_app| {
            tauri::async_runtime::spawn(async move {
                proxy::start_http_proxy(7890, http_upstream).await.unwrap();
            });
            tauri::async_runtime::spawn(async move {
                proxy::start_socks5_proxy(7891, socks_upstream).await.unwrap();
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
            check_proxy_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
