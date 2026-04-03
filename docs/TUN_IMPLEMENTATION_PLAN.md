# TUN 功能实现方案说明

## 📋 当前实现方案

### 技术选型

使用 `tun` crate 实现 TUN 设备管理。

### 优点

- ✅ 跨平台支持（macOS/Linux/Windows）
- ✅ 使用 Rust 异步 API
- ✅ 集成到 Tauri 应用

### 缺点

- ⚠️ **需要管理员权限**（这是主要限制）

## 🔐 权限要求

### macOS

**当前要求**: 需要使用 sudo 运行

```bash
sudo open -a xnetify
# 或
sudo /Applications/xnetify.app/Contents/MacOS/xnetify
```

**原因**: 
- tun crate 需要直接访问 `/dev/tun` 设备
- macOS 的沙盒安全机制限制普通应用访问网络设备

### Linux

```bash
sudo ./xnetify
```

### Windows

以管理员身份运行应用。

## 🎯 ClashX 方案对比

ClashX 等应用使用的是 **System Extension**（系统扩展）方案，这是 macOS 原生的推荐方式。

### System Extension 方案

**优点**:
- ✅ 用户只需要点击一次"允许"
- ✅ 不需要每次都输入密码
- ✅ 授权一次后永久有效
- ✅ Apple 官方推荐的方案

**缺点**:
- ❌ 实现复杂，需要编写 Swift 代码
- ❌ 需要配置代码签名
- ❌ 需要 Apple Developer 账号
- ❌ 需要创建独立的 Extension target

### 实现步骤

1. **创建 System Extension Target**
   - 使用 Xcode 创建 Network Extension
   - 实现 NEPacketTunnelProvider

2. **代码签名配置**
   - 配置 entitlements
   - 启用 com.apple.developer.networking.networkextension capability

3. **集成到 Tauri**
   - 使用 Swift 编写 Tauri plugin
   - 调用 System Extension API

## 📊 方案对比

| 方案 | 权限要求 | 用户体验 | 实现复杂度 |
|------|---------|---------|-----------|
| **tun crate（当前）** | 需要 sudo | 繁琐 | 简单 |
| **System Extension** | 授权一次 | 流畅 | 复杂 |
| **混合方案** | 部分需要 sudo | 中等 | 中等 |

## 🔄 未来的改进方向

### 方案 1: System Extension（推荐）

这是 macOS 上最正确的做法，但需要较大的开发投入。

**实现时间**: 预计 2-4 周

**需要的资源**:
- Apple Developer 账号（年费 $99）
- Swift 编程经验
- 理解 NetworkExtension framework

### 方案 2: 混合方案

同时支持 tun crate 和 System Extension，根据平台自动选择。

**实现时间**: 预计 1-2 周

**逻辑**:
```
macOS:
  尝试使用 System Extension
  如果失败，使用 tun crate
  如果都失败，提示使用系统代理

Linux/Windows:
  使用 tun crate
```

### 方案 3: 简化当前方案

专注于系统代理模式，TUN 作为可选高级功能。

**实现时间**: 无需改动

**用户指南**:
- 提供详细的权限获取说明
- 视频教程
- FAQ 文档

## 💡 当前建议

### 短期（立即）

1. **完善文档**
   - 详细的权限获取指南
   - 视频教程
   - 常见问题解答

2. **优化提示**
   - 清晰的错误信息
   - 备用方案提示
   - 一键复制命令

### 中期（1-2个月）

1. **混合方案**
   - 先使用 System Extension
   - 失败时回退到 tun crate

2. **完善测试**
   - 在不同 macOS 版本测试
   - 不同硬件配置测试

### 长期（3-6个月）

1. **System Extension**
   - 创建独立的 Extension target
   - 实现完整的 NEPacketTunnelProvider
   - 配置代码签名

2. **发布准备**
   - Apple Developer 账号注册
   - App Store 准备（如果需要）
   - 权限申请

## 📚 参考资源

### Apple 官方文档
- [System Extension 文档](https://developer.apple.com/documentation/systemextensions)
- [Network Extension](https://developer.apple.com/documentation/networkextension)
- [NEPacketTunnelProvider](https://developer.apple.com/documentation/networkextension/nepackettunnelprovider)

### 开源项目参考
- [ClashX](https://github.com/yichengchen/clashX)
- [Mihomo Party](https://github.com/ExpMihoParty/Mihomo-Party)
- [ObscuraVPN](https://github.com/Sovereign-Engineering/obscuravpn-client)

### Rust 相关
- [objc2-network-extension](https://docs.rs/objc2-network-extension/latest/objc2_network_extension/)
- [tun crate](https://docs.rs/tun/latest/tun/)

## 🎯 结论

当前使用 `tun` crate 的方案：
- ✅ 实现简单，快速可用
- ⚠️ 需要用户手动获取权限
- ⚠️ 体验不如 System Extension

**推荐路径**:
1. **立即**: 完善文档，优化提示
2. **中期**: 实现混合方案
3. **长期**: 迁移到 System Extension

---

*了解限制，持续改进。*
