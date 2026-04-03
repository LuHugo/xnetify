# TUN 功能当前状态说明

## ⚠️ 重要说明

### macOS 上的 TUN 限制

在 macOS 上，使用 `tun` crate 创建 TUN 设备存在以下限制：

1. **必须使用 root 权限**
   - `tun` crate 需要直接访问 `/dev/tun` 设备
   - 普通用户权限会导致 `Permission denied` 错误
   - **目前无法通过 UI 自动提权到 root**

2. **当前权限提升方案的局限性**
   - 我们实现的 `request_elevation()` 使用 AppleScript
   - AppleScript 可以请求管理员认证，但**无法将当前应用提升为 root**
   - 它只能启动一个新的管理员进程，而不是提升现有进程

### 为什么无法自动提权？

```bash
# AppleScript 只能做到：
osascript -e "do shell script \"open -a xnetify\" with administrator privileges"
# 这会打开一个新的 xnetify 窗口（需要密码）
# 但原来的窗口仍然是普通权限
```

macOS 的安全机制不允许一个普通权限的进程自动提升到 root。

## 🎯 解决方案

### 方案 1: 用户手动以 sudo 运行（当前推荐）

```bash
# macOS
sudo open -a xnetify

# Linux
sudo ./xnetify

# Windows
# 右键 > 以管理员身份运行
```

### 方案 2: 创建带有 setuid 的启动脚本

```bash
#!/bin/bash
# 创建 sudo-wrapper 脚本
sudo chown root:admin /Applications/xnetify.app/Contents/MacOS/xnetify
sudo chmod +s /Applications/xnetify.app/Contents/MacOS/xnetify
```

⚠️ 这有安全风险，不推荐。

### 方案 3: 使用 System Extension（长期方案）

这是 macOS 上最正确的做法：

1. 使用 Apple 的 System Extension Framework
2. 用户只需要点击一次"允许"
3. 不需要每次都输入密码
4. 参考 ClashX、Mihomo Party 实现

**这是未来的最佳方案，但需要较多开发工作。**

## 📋 当前已实现的功能

✅ **权限检测**
- 自动检测当前是否有管理员权限
- 每 30 秒定期检测
- UI 显示权限状态指示器

✅ **友好的错误提示**
- 清晰的错误信息
- 平台特定的指导
- "去设置"按钮

✅ **兜底方案**
- TUN 失败时提示切换到系统代理模式
- 确保软件可用

❌ **自动提权** - 不支持（macOS 安全限制）

## 🔧 改进的错误信息

现在后端会返回更清晰的错误信息：

```rust
// 当权限不足时
"需要管理员权限

在 macOS 上，请打开终端并运行:
sudo /Applications/xnetify.app/Contents/MacOS/xnetify"
```

```rust
// 当 TUN 设备创建失败时
"TUN 设备创建失败: Permission denied

提示: 权限不足：需要 root 权限来创建 TUN 设备。请使用 sudo 运行应用。"
```

## 🎨 UI 改进

### 权限状态指示器

```
开启TUN  [⚠️ 需要权限] [去获取]  [开关]  ← 红色警告
开启TUN  [✅ 已就绪]            [开关]  ← 绿色就绪
```

### 错误提示

```
┌─────────────────────────────────┐
│  需要管理员权限                   │
│                                  │
│  在 macOS 上，请打开终端并运行:   │
│  sudo /Applications/xnetify...  │
│                                  │
│  ┌─────────────┐ ┌───────┐     │
│  │  打开终端   │ │  取消  │     │
│  └─────────────┘ └───────┘     │
└─────────────────────────────────┘
```

## 📝 用户使用流程

### 当前（推荐方式）

1. 用户打开应用（普通权限）
2. ⚠️ 看到权限指示器显示"需要权限"
3. 用户点击 TUN 开关
4. 弹出提示，告知需要 sudo
5. 用户打开终端，输入 `sudo open -a xnetify`
6. 在新的管理员窗口中使用 TUN 功能

### 理想流程（未来）

1. 用户打开应用（普通权限）
2. ⚠️ 看到权限指示器显示"需要权限"
3. 用户点击 TUN 开关
4. 弹出系统授权对话框
5. 用户输入密码
6. ✅ TUN 功能自动启动

## 🔮 后续计划

### 短期
- [ ] 改进错误提示（已完成）
- [ ] 添加详细的测试文档（已完成）
- [ ] 优化权限指示器

### 中期
- [ ] 研究 System Extension 实现
- [ ] 参考 ClashX/Mihomo Party 代码
- [ ] 设计权限管理架构

### 长期
- [ ] 使用 System Extension 替代 tun crate
- [ ] 实现自动化权限配置
- [ ] 提供更好的用户体验

## 📚 参考资料

- [ClashX 实现](https://github.com/yichengchen/clashX)
- [Mihomo Party 实现](https://github.com/ExpoMaa/Mihomo-Party)
- [System Extension 文档](https://developer.apple.com/documentation/systemextensions)
- [tun crate issues](https://github.com/meh/rust-tun/issues)

## ⚠️ 注意事项

1. **安全第一**
   - 不要为了方便而降低安全标准
   - root 权限需要谨慎使用

2. **用户体验平衡**
   - 尽可能自动化
   - 但不要绕过安全机制

3. **跨平台考虑**
   - macOS 最复杂（沙盒 + System Extension）
   - Linux 相对简单（sudo 即可）
   - Windows 有 UAC，相对成熟

---

*了解限制，持续改进。*
