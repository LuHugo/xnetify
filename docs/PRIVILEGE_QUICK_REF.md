# 权限提升功能快速参考

## 🎯 功能概述

当用户点击"TUN 开启"时自动检测权限，权限不足时弹出系统提权对话框。

## 🔧 API

### Rust 命令

```rust
// 检查是否具有管理员权限
check_admin() -> PrivilegeResult

// 请求权限提升
request_elevation() -> PrivilegeResult

// 获取权限提示
get_elevation_message() -> String
```

### 前端方法

```typescript
// 检查权限
await checkPrivilege(): Promise<boolean>

// 请求提权
await requestElevation(): Promise<PrivilegeResult>
```

## 📦 PrivilegeResult

```typescript
interface PrivilegeResult {
  success: boolean;       // 是否成功
  needs_restart: boolean;  // 是否需要重启
  message: string;        // 信息或错误
}
```

## 🎨 使用流程

```
用户点击 TUN 开关
    ↓
检查权限 isAdmin
    ↓
┌──────────┐     ┌──────────┐
│ 没有权限  │ ──→ │ 有权限    │
└──────────┘     └──────────┘
    ↓                  ↓
显示提权提示      直接启动 TUN
    ↓
点击"获取权限"
    ↓
调用 request_elevation()
    ↓
┌──────────────────────────┐
│ macOS: AppleScript 对话框│
│ Linux: 显示手动指南      │
│ Windows: 启动新进程 (UAC)│
└──────────────────────────┘
    ↓
成功？──┬──→ 是 → 重启应用（管理员）
        │
        └──→ 否 → 显示错误信息
```

## 📋 平台支持

| 平台 | 提权方式 | UI | 状态 |
|------|---------|-----|------|
| macOS | AppleScript | 系统对话框 | ✅ |
| Linux | 显示指南 | Toast 提示 | ✅ |
| Windows | PowerShell | UAC | ✅ |

## 🔍 测试命令

```bash
# Rust 编译
cd src-tauri && cargo build

# 前端编译
npm run build

# 开发模式
npm run tauri dev
```

## 📁 关键文件

- `src-tauri/src/privilege.rs` - 权限管理
- `src-tauri/src/tun.rs` - TUN 集成权限检查
- `src/store/index.ts` - 前端权限状态
- `src/App.tsx` - 权限 UI

## ⚠️ 注意事项

1. macOS 需要输入密码
2. Windows 会创建新的进程实例
3. Linux 显示手动指南
4. 提权可能需要重启应用

## 📚 文档

- 详细文档: `docs/PRIVILEGE_ESCALATION.md`
- TUN 功能: `docs/TUN_*.md`
