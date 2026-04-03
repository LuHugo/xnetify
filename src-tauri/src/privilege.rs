use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeResult {
    pub success: bool,
    pub needs_restart: bool,
    pub message: String,
}

pub fn is_admin() -> bool {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("id")
            .arg("-u")
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim() == "0"
            }
            Err(_) => false,
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("id")
            .arg("-u")
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim() == "0"
            }
            Err(_) => false,
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let output = Command::new("net")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .args(["session"])
            .output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

pub fn get_platform_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS";

    #[cfg(target_os = "linux")]
    return "Linux";

    #[cfg(target_os = "windows")]
    return "Windows";

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return "Unknown";
}

pub fn get_elevation_command() -> Option<(String, Vec<String>)> {
    #[cfg(target_os = "macos")]
    {
        Some((
            "osascript".to_string(),
            vec![
                "-e".to_string(),
                "do shell script \"printf '%s' $$\" with administrator privileges".to_string(),
            ],
        ))
    }

    #[cfg(target_os = "linux")]
    {
        if Command::new("pkexec").arg("--version").output().is_ok() {
            Some(("pkexec".to_string(), vec![]))
        } else if Command::new("gksu").arg("--version").output().is_ok() {
            Some(("gksu".to_string(), vec![]))
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    {
        Some((
            "powershell".to_string(),
            vec![
                "-Command".to_string(),
                "Start-Process -FilePath powershell -Verb RunAs -ArgumentList '-Command Start-Process xnetify.exe -Verb RunAs'".to_string(),
            ],
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

pub fn get_elevation_instructions() -> String {
    let platform = get_platform_name();
    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "xnetify".to_string());

    match platform {
        "macOS" => {
            format!(
                "请使用 sudo 运行应用：\n1. 打开终端\n2. 运行: sudo \"{}\"",
                exe_path
            )
        }
        "Linux" => {
            format!(
                "请使用 sudo 运行应用：\n1. 打开终端\n2. 运行: sudo \"{}\"",
                exe_path
            )
        }
        "Windows" => {
            "请以管理员身份运行：\n1. 右键点击 xnetify.exe\n2. 选择\"以管理员身份运行\"".to_string()
        }
        _ => {
            format!("请以管理员/root 权限运行: {}", exe_path)
        }
    }
}

#[tauri::command]
pub fn check_admin() -> PrivilegeResult {
    let is_admin = is_admin();

    PrivilegeResult {
        success: is_admin,
        needs_restart: false,
        message: if is_admin {
            "已拥有管理员权限".to_string()
        } else {
            "需要管理员权限".to_string()
        },
    }
}

#[tauri::command]
pub async fn request_elevation() -> PrivilegeResult {
    if is_admin() {
        return PrivilegeResult {
            success: true,
            needs_restart: false,
            message: "已拥有管理员权限".to_string(),
        };
    }

    let platform = get_platform_name();

    #[cfg(target_os = "macos")]
    {
        let exe_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "xnetify".to_string());
        
        let script = format!(
            r#"do shell script "sudo '{}'" with administrator privileges"#,
            exe_path.replace("'", "'\\''")
        );

        match Command::new("osascript")
            .args(["-e", &script])
            .spawn()
        {
            Ok(_) => {
                PrivilegeResult {
                    success: true,
                    needs_restart: true,
                    message: "正在以管理员身份重启应用...".to_string(),
                }
            }
            Err(e) => PrivilegeResult {
                success: false,
                needs_restart: false,
                message: format!("提权失败: {}", e),
            },
        }
    }

    #[cfg(target_os = "linux")]
    {
        let instructions = get_elevation_instructions();
        let result = Command::new("pkexec")
            .args(["--disable-internal-agent", "--version"])
            .output();

        match result {
            Ok(_) => {
                PrivilegeResult {
                    success: false,
                    needs_restart: true,
                    message: "请使用 sudo 运行应用，或在终端中运行: pkexec xnetify".to_string(),
                }
            }
            Err(_) => {
                PrivilegeResult {
                    success: false,
                    needs_restart: false,
                    message: instructions,
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "xnetify.exe".to_string());

        let ps_script = format!(
            r#"Start-Process -FilePath "{}" -Verb RunAs"#,
            current_exe
        );

        match Command::new("powershell")
            .args(["-Command", &ps_script])
            .spawn()
        {
            Ok(_) => PrivilegeResult {
                success: true,
                needs_restart: true,
                message: "正在以管理员身份重启应用...".to_string(),
            },
            Err(e) => PrivilegeResult {
                success: false,
                needs_restart: false,
                message: format!("提权失败: {}", e),
            },
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PrivilegeResult {
            success: false,
            needs_restart: false,
            message: get_elevation_instructions(),
        }
    }
}

#[tauri::command]
pub fn get_elevation_message() -> String {
    get_elevation_instructions()
}
