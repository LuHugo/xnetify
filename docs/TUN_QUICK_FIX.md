# TUN 功能快速修复指南

## 🚀 立即解决问题

### macOS

**打开终端，运行以下命令：**

```bash
# 方式 1: 使用 sudo 打开应用
sudo open -a xnetify

# 方式 2: 直接运行
sudo /Applications/xnetify.app/Contents/MacOS/xnetify
```

输入密码后，应用将以管理员权限运行，TUN 功能即可正常使用。

---

### Linux

```bash
# 运行应用
sudo ./xnetify

# 或者
sudo -i ./xnetify
```

---

### Windows

1. 右键点击 `xnetify.exe`
2. 选择 **"以管理员身份运行"**

---

## 🔍 验证是否成功

### 权限指示器

在应用界面查看 TUN 开关旁边：

- ✅ **已就绪**（绿色）= 有权限，可以使用 TUN
- ⚠️ **需要权限**（红色）= 没有权限

### 测试 TUN

1. 确保有一个可用的代理应用（已测试通过）
2. 点击 TUN 开关
3. 应该显示 "TUN 模式已开启"

---

## ❌ 如果还是失败

### 检查日志

运行应用时查看终端输出：

```bash
sudo RUST_LOG=debug /Applications/xnetify.app/Contents/MacOS/xnetify
```

寻找以下信息：
- `TUN 设备创建成功`
- `TUN 设备创建失败`
- `Permission denied`

### 常见错误

| 错误 | 原因 | 解决方案 |
|------|------|---------|
| Permission denied | 没有 root 权限 | 使用 sudo 运行 |
| Device busy | TUN 设备被占用 | 重启应用 |
| Address in use | IP 地址冲突 | 等待或重启 |

---

## 💡 提示

### macOS 用户

如果不想每次都输入密码：

1. 打开 **系统偏好设置** > **安全性与隐私**
2. 点击左下角的 🔒 解锁
3. 允许 "xnetify" 应用

这样只需要输入一次密码。

### Windows 用户

可以将应用固定到任务栏，然后：
1. 右键任务栏图标
2. 右键 "xnetify"
3. 选择 "以管理员身份运行"
4. 以后双击任务栏图标就会以管理员身份启动

---

## 📞 需要帮助？

如果以上方法都不行：

1. 记下错误信息
2. 查看详细文档：`docs/TUN_TESTING.md`
3. 查看当前限制：`docs/TUN_MACOS_LIMITATIONS.md`

---

*快速开始，尽情使用！*
