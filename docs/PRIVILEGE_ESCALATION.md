# 跨平台权限提升功能实现

## 🎯 实现概述

当用户点击"TUN 开启"时，系统会检测是否具有管理员权限：
- 如果已有权限 → 直接启动 TUN
- 如果没有权限 → 弹出系统原生权限提升对话框
- 提权成功 → 以管理员身份重启应用

## ✅ 完成的功能

### 1. Rust 后端 (`src-tauri/src/privilege.rs`)

- ✅ `is_admin()` - 检查当前是否具有管理员权限
- ✅ `get_platform_name()` - 获取当前操作系统名称
- ✅ `get_elevation_instructions()` - 获取平台特定的权限获取说明
- ✅ `check_admin()` - Tauri 命令：检查管理员权限
- ✅ `request_elevation()` - Tauri 命令：请求权限提升
- ✅ `get_elevation_message()` - Tauri 命令：获取权限提示信息

### 2. Tauri 命令

```rust
// 检查是否具有管理员权限
#[tauri::command]
pub fn check_admin() -> PrivilegeResult

// 请求权限提升
#[tauri::command]
pub async fn request_elevation() -> PrivilegeResult

// 获取权限提示
#[tauri::command]
pub fn get_elevation_message() -> String
```

### 3. TUN 集成 (`src-tauri/src/tun.rs`)

- ✅ 在 `tun_start` 中自动检查权限
- ✅ 权限不足时返回友好的错误信息
- ✅ 提供平台特定的权限获取指南

### 4. 前端 Store (`src/store/index.ts`)

- ✅ 添加 `isAdmin` 状态
- ✅ 添加 `checkPrivilege()` - 检查权限
- ✅ 添加 `requestElevation()` - 请求提权
- ✅ 类型定义 `PrivilegeResult`

### 5. 前端 UI (`src/App.tsx`)

- ✅ 应用启动时自动检查权限
- ✅ TUN 开关自动处理权限检查
- ✅ 权限不足时显示友好的 Toast 提示
- ✅ 提供"获取权限"按钮，一键提权
- ✅ 提权后引导用户重启应用

## 🔧 工作流程

### 启动 TUN 流程

```
用户点击 TUN 开关
    ↓
检查是否具有管理员权限
    ↓
┌─────────────────────┐
│ 没有权限             │
└─────────────────────┘
    ↓
显示权限提示对话框
    ↓
用户点击"获取权限"
    ↓
调用 request_elevation()
    ↓
┌─────────────────────┐
│ macOS               │
│ 使用 AppleScript    │
│ 弹出认证对话框      │
└─────────────────────┘
    ↓
┌─────────────────────┐
│ Linux              │
│ 显示手动提权指南    │
└─────────────────────┘
    ↓
┌─────────────────────┐
│ Windows            │
│ 启动新进程（UAC）  │
└─────────────────────┘
    ↓
提示用户重启应用
```

## 📦 PrivilegeResult 结构

```rust
pub struct PrivilegeResult {
    pub success: bool,       // 是否成功获取权限
    pub needs_restart: bool, // 是否需要重启应用
    pub message: String,     // 结果信息或错误提示
}
```

## 🎨 前端交互

### 权限检查 Toast

```typescript
toast.info('需要管理员权限', {
    description: '正在请求权限提升...',
    action: {
        label: '获取权限',
        onClick: async () => {
            const result = await requestElevation();
            if (result.success) {
                toast.success('权限提升成功');
                if (result.needs_restart) {
                    toast.info('请在新的管理员窗口中使用 TUN 功能');
                }
            } else {
                toast.error(result.message);
            }
        }
    },
    duration: 10000
});
```

### 错误处理

自动检测权限相关错误并显示友好提示：

```typescript
if (errorMessage.includes('管理员权限') || 
    errorMessage.includes('Permission denied')) {
    toast.error('权限不足', {
        description: '...',
        action: { label: '获取权限', onClick: ... },
        duration: 15000
    });
}
```

## 🖥️ 平台特定实现

### macOS

**提权方法**: AppleScript
```rust
osascript -e "do shell script \"...\" with administrator privileges"
```

**用户提示**: 
```
请使用 sudo 运行应用：
1. 打开终端
2. 运行: sudo /Applications/xnetify.app/Contents/MacOS/xnetify
```

### Linux

**提权方法**: 显示手动指南（pkexec 可能有兼容性问题）
```bash
# 检测可用工具
pkexec --version  # 可用
gksu --version    # 可用
```

**用户提示**:
```
请使用 sudo 运行应用：
1. 打开终端
2. 运行: sudo xnetify
```

### Windows

**提权方法**: PowerShell 启动新进程
```powershell
Start-Process -FilePath "xnetify.exe" -Verb RunAs
```

**用户提示**:
```
请以管理员身份运行：
1. 右键点击 xnetify.exe
2. 选择"以管理员身份运行"
```

## 📝 文件变更

### 新增文件
- `src-tauri/src/privilege.rs` - 权限管理模块

### 修改文件
- `src-tauri/Cargo.toml` - 移除无效依赖
- `src-tauri/src/lib.rs` - 集成权限命令
- `src-tauri/src/tun.rs` - 集成权限检查
- `src/store/index.ts` - 添加权限状态和方法
- `src/App.tsx` - 添加权限提示 UI

## 🔍 测试清单

- [ ] macOS 权限检查
- [ ] macOS AppleScript 提权对话框
- [ ] Linux 权限检查
- [ ] Linux 提权指南显示
- [ ] Windows 权限检查
- [ ] Windows UAC 提权
- [ ] 权限不足时的错误提示
- [ ] 提权成功后的用户引导
- [ ] TUN 启动流程
- [ ] TUN 停止流程

## 📊 编译状态

✅ Rust 后端编译成功
✅ 前端编译成功

## ⚠️ 已知问题

1. **Linux 提权**: 不同桌面环境可能需要不同的提权工具
2. **macOS 提权**: 需要用户输入密码
3. **Windows 提权**: 会创建新的进程实例

## 🔮 未来改进

1. 支持更多 Linux 桌面环境的自动提权
2. 添加权限状态指示器（图标）
3. 权限引导教程（首次使用）
4. 设置页面权限管理
5. 权限持久化配置

## 📚 参考资料

- [Tauri 权限系统](https://tauri.app/security/permissions/)
- [AppleScript 权限](https://developer.apple.com/library/archive/documentation/LegacyTechnologies/Talks/applescriptlangref/pdf/AppleScriptLangRef.pdf)
- [PowerShell RunAs](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.management/start-process)
- [pkexec Manual](https://www.freedesktop.org/software/polkit/docs/latest/pkexec.1.html)

---

*最后更新: 2026年3月31日*
