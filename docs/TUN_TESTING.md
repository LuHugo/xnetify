# TUN 功能测试指南

## 🔍 当前状态

TUN 功能在 macOS 上需要特殊的权限配置才能正常工作。

## ⚠️ 已知问题

### macOS

1. **需要 root 权限**
   - tun crate 在 macOS 上创建设备需要 root 权限
   - 普通用户权限会导致 "Permission denied" 错误

2. **设备名称**
   - macOS 使用 `utun` 系列设备名
   - 系统自动分配（如 utun0, utun1）

### Linux

1. **需要 root 权限**
   - 需要 root 或 CAP_NET_ADMIN 权限

2. **TUN 设备文件**
   - 需要 `/dev/net/tun` 设备存在

### Windows

1. **需要管理员权限**
   - 使用 Wintun 驱动

## 🧪 测试步骤

### 1. 以管理员权限运行应用

#### macOS

```bash
# 方式 1: 使用 sudo 启动
sudo /Applications/xnetify.app/Contents/MacOS/xnetify

# 方式 2: 使用 open 命令
sudo open -a xnetify
```

#### Linux

```bash
sudo ./target/release/xnetify
# 或
sudo -i xnetify
```

#### Windows

1. 右键点击 xnetify.exe
2. 选择"以管理员身份运行"

### 2. 检查权限状态

启动应用后，查看 TUN 开关旁边的权限指示器：
- ✅ 已就绪 - 表示有权限
- ⚠️ 需要权限 - 表示没有权限

### 3. 测试 TUN 启动

1. 确保有一个可用的代理应用（已测试通过）
2. 点击 TUN 开关
3. 观察结果

## 📊 预期结果

### 成功情况

```
✅ 权限指示器显示: "已就绪"
✅ 点击 TUN 开关
✅ 显示: "TUN 模式已开启"
✅ 可以在日志中看到: "TUN 设备创建成功"
```

### 失败情况（权限不足）

```
⚠️ 权限指示器显示: "需要权限"
❌ 点击 TUN 开关
❌ 显示: "需要管理员权限"
❌ 日志显示: "Permission denied"
```

### 失败情况（其他错误）

```
✅ 权限指示器显示: "已就绪"
❌ 点击 TUN 开关
❌ 显示: "TUN 设备创建失败"
❌ 日志显示具体错误信息
```

## 🔧 调试技巧

### 1. 查看详细日志

运行应用时查看终端输出：
```bash
sudo RUST_LOG=debug /Applications/xnetify.app/Contents/MacOS/xnetify
```

### 2. 手动测试 TUN 设备

macOS:
```bash
# 查看现有 TUN 设备
ifconfig | grep utun

# 手动创建测试
sudo touch /dev/tun0
sudo chmod 666 /dev/tun0
```

Linux:
```bash
# 检查 TUN 设备
ls -la /dev/net/tun

# 检查权限
cat /proc/sys/net/ipv4/ip_forward
```

### 3. 测试权限

```bash
# 检查当前用户 ID
id

# 应该显示 uid=0 表示 root
# uid=501 表示普通用户
```

## 📝 错误信息对照表

| 错误信息 | 原因 | 解决方案 |
|---------|------|---------|
| Permission denied | 权限不足 | 使用 sudo 运行 |
| Device or resource busy | 设备被占用 | 重启应用或释放设备 |
| No such file or directory | TUN 驱动未安装 | 安装 TUN 驱动 |
| Invalid argument | 配置错误 | 检查 TUN 配置 |
| Address already in use | IP 地址冲突 | 使用不同的 IP |

## 🎯 最佳实践

### macOS

1. **首次使用**
   ```bash
   # 安装应用
   open -a xnetify
   
   # 以 sudo 运行
   sudo open -a xnetify
   ```

2. **权限持久化**
   - 在"系统偏好设置" > "安全性与隐私"中允许应用

### Linux

1. **使用 sudo**
   ```bash
   sudo ./xnetify
   ```

2. **检查权限**
   ```bash
   # 检查是否 root
   whoami  # 应该输出: root
   
   # 检查 TUN 能力
   cat /proc/sys/net/ipv4/ip_forward
   ```

### Windows

1. **以管理员运行**
   - 右键 > 以管理员身份运行
   - 或在 PowerShell 中: `Start-Process xnetify.exe -Verb RunAs`

## 🔄 替代方案

如果 TUN 模式无法正常工作，可以使用**系统代理模式**作为替代：

1. 点击 TUN 开关失败后，选择"切换到代理模式"
2. 系统代理模式不需要特殊权限
3. 可以正常代理 HTTP/HTTPS 流量

## 📞 获取帮助

如果遇到问题，请提供以下信息：

1. 操作系统版本
2. 应用日志（终端输出）
3. 错误信息截图
4. 已尝试的解决方法

## 🔮 未来改进

计划中的改进：
1. 使用 System Extension 替代 tun crate
2. 自动化权限配置
3. 更好的错误提示
4. 参考 ClashX/Mihomo Party 实现

---

*最后更新: 2026年3月31日*
