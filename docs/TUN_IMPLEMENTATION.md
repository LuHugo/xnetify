# TUN 功能实现文档

## 概述

TUN 模块是 xnetify 的高级功能，允许用户将系统所有流量路由到 TUN 设备，实现透明代理。

## 架构

```
系统流量 → TUN 设备 → xnetify → VPN 应用
                    ↓
               DNS 劫持处理
                    ↓
              流量转发逻辑
```

## 实现的功能

### ✅ 已完成

1. **TUN 设备管理**
   - 创建和销毁 TUN 设备
   - 异步读写数据包
   - 设备状态管理

2. **流量处理**
   - 从 TUN 读取 IP 数据包
   - 分析数据包类型（DNS、TCP、UDP）
   - 转发到上游 VPN
   - 接收响应并写回 TUN

3. **DNS 劫持**
   - 检测 DNS 请求（UDP 端口 53）
   - 生成 DNS 响应
   - 支持自定义 DNS 服务器

4. **Tauri 命令接口**
   - `tun_start`: 启动 TUN 模式
   - `tun_stop`: 停止 TUN 模式
   - `tun_get_status`: 获取状态
   - `tun_update_upstream`: 更新上游配置

### ⬜ 待实现

1. **系统路由配置**
   - 自动配置系统路由表
   - 添加默认路由到 TUN 设备
   - 需要管理员权限

2. **DNS 系统配置**
   - macOS: networksetup
   - Linux: /etc/resolv.conf 或 resolvectl
   - Windows: netsh

3. **高级功能**
   - 多 VPN 应用支持
   - 流量统计和监控
   - 连接管理
   - 路由规则配置
   - 前端 UI 集成

## 使用方法

### 前端调用

```typescript
import { tunStart, tunStop, tunGetStatus } from './tun-api';

// 启动 TUN 模式
await tunStart('vpn-server.com', 443);

// 获取状态
const status = await tunGetStatus();

// 停止 TUN 模式
await tunStop();
```

### 配置参数

```rust
struct TunConfig {
    device_name: String,       // 自动分配
    tunnel_ip: String,        // 默认 10.0.0.2
    tunnel_netmask: String,   // 默认 255.255.255.0
    dns_server: String,       // 默认 8.8.8.8
    mtu: u16,               // 默认 1500
    upstream_host: String,    // VPN 服务器地址
    upstream_port: u16,       // VPN 服务器端口
}
```

## 权限要求

- **macOS**: 需要 root 权限或 sudo
- **Linux**: 需要 root 或 CAP_NET_ADMIN 权限
- **Windows**: 需要管理员权限

## 文件结构

```
src-tauri/src/
├── tun.rs              # TUN 模块实现
├── tun_example.rs      # 使用示例和测试
├── proxy.rs           # 现有代理功能
└── lib.rs             # 主入口

src/components/
└── TunExample.tsx     # 前端调用示例
```

## 技术细节

### 依赖

```toml
tun = { version = "0.8", features = ["async"] }
ipnetwork = "0.20"
log = "0.4"
env_logger = "0.11"
```

### 关键类型

- `TunManager`: TUN 设备管理器
- `TunConfig`: 配置结构
- `TunStatus`: 状态结构
- `TunError`: 错误类型

### 数据包处理流程

1. 从 TUN 设备异步读取数据包
2. 检查是否为 DNS 请求（UDP 端口 53）
3. 如果是 DNS 请求，生成响应并直接返回
4. 否则，通过 TCP 连接到上游 VPN
5. 发送数据包到 VPN，接收响应
6. 将响应写回 TUN 设备

## 注意事项

1. **权限**: 创建 TUN 设备需要系统权限
2. **平台差异**: 不同平台使用不同的 TUN 设备名称
   - macOS: utunX
   - Linux: tunX
   - Windows: Wintun
3. **上游连接**: 当前实现假设上游是 TCP 连接
4. **DNS 劫持**: 基础实现，直接返回硬编码的 DNS 响应

## 未来改进

1. 支持 SOCKS5/HTTP 代理上游
2. 实现真正的 DNS 解析
3. 添加路由规则引擎
4. 支持流量监控和统计
5. 实现连接池管理
6. 添加错误重试机制
7. 前端 UI 集成

## 测试

```bash
# 编译检查
cargo build

# 运行测试
cargo test

# 文档生成
cargo doc --no-deps
```

## 许可

继承 xnetify 项目许可。
