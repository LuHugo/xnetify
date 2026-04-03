# TUN 功能前端集成说明

## 实现日期
2026年3月31日

## 功能概述
在 xnetify 前端添加了 TUN 模式控制功能，用户可以通过 UI 开关来启用/禁用 TUN 模式。

## 实现细节

### 1. Store 更新 (`src/store/index.ts`)

#### 新增类型定义
```typescript
interface TunConfig {
  device_name: string;
  tunnel_ip: string;
  tunnel_netmask: string;
  dns_server: string;
  mtu: number;
  upstream_host: string;
  upstream_port: number;
}

interface TunStatus {
  active: boolean;
  config: TunConfig | null;
  packets_in: number;
  packets_out: number;
  error: string | null;
}
```

#### 新增状态
- `tunEnabled: boolean` - TUN 模式是否启用
- `tunStatus: TunStatus | null` - TUN 详细状态

#### 新增方法
- `toggleTun(enabled: boolean)` - 切换 TUN 模式
- `checkTunStatus()` - 检查 TUN 状态
- `updateTunUpstream(host: string, port: number)` - 更新上游 VPN

### 2. App.tsx 更新 (`src/App.tsx`)

#### UI 组件
已有的 TUN 开关组件现在已连接到后端逻辑：
```tsx
<Item variant="muted">
  <ItemContent>
    <ItemHeader>开启TUN</ItemHeader>
  </ItemContent>
  <ItemActions>
    <Switch 
      checked={tunEnabled}
      onCheckedChange={handleTunSwitch} 
      className={cn(tunEnabled && "data-checked:bg-green-500")}
    />
  </ItemActions>
</Item>
```

#### 处理函数
```typescript
const handleTunSwitch = async (checked: boolean) => {
  try {
    await toggleTun(checked);
    toast.success(checked ? "TUN 模式已开启" : "TUN 模式已关闭");
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : '未知错误';
    toast.error(checked ? `开启 TUN 失败: ${errorMessage}` : "关闭 TUN 失败");
  }
};
```

#### 自动状态检查
在应用启动时自动检查 TUN 状态：
```typescript
useEffect(() => {
  detectProxies();
  checkProxyStatus();
  checkTunStatus(); // 新增
  const interval = setInterval(() => {
    silentTestProxies();
  }, 5 * 60 * 1000);
  return () => clearInterval(interval);
}, [..., checkTunStatus]); // 新增依赖
```

## 工作流程

### 启动 TUN 模式
1. 用户点击 TUN 开关
2. 前端查找第一个可用的代理应用
3. 调用 `tun_start` 命令，传入上游地址和端口
4. 后端创建 TUN 设备并开始流量转发
5. 前端显示成功提示并更新状态

### 停止 TUN 模式
1. 用户关闭 TUN 开关
2. 前端调用 `tun_stop` 命令
3. 后端停止数据包处理器并销毁设备
4. 前端显示成功提示并重置状态

### 上游选择逻辑
```typescript
// 找到第一个测试通过的代理作为上游
const upstreamApp = apps.find(app => app.tested);
if (!upstreamApp) {
  throw new Error('没有可用的代理应用');
}

const upstreamHost = upstreamApp.host;
const upstreamPort = upstreamApp.socks_port || upstreamApp.http_port || 1080;
```

## 用户体验

### 成功场景
- ✅ TUN 开启成功：显示 "TUN 模式已开启" toast
- ✅ TUN 关闭成功：显示 "TUN 模式已关闭" toast
- ✅ 状态同步：应用重启后自动恢复 TUN 状态

### 错误场景
- ❌ 没有可用代理：显示 "没有可用的代理应用"
- ❌ 权限不足：显示 "需要管理员/root 权限"
- ❌ 设备创建失败：显示具体错误信息

## 技术细节

### 依赖
- Tauri 后端 `tun` crate (async feature)
- 前端使用 Zustand store 管理状态

### API 调用
```typescript
// 启动 TUN
await invoke('tun_start', { 
  host: '127.0.0.1', 
  port: 1080 
});

// 停止 TUN
await invoke('tun_stop');

// 获取状态
await invoke<TunStatus>('tun_get_status');

// 更新上游
await invoke('tun_update_upstream', { 
  host: 'new-host.com', 
  port: 443 
});
```

## 权限要求

TUN 设备创建需要系统权限：
- **macOS**: root 权限或 sudo
- **Linux**: root 或 CAP_NET_ADMIN
- **Windows**: 管理员权限

## 后续改进建议

1. **状态显示增强**
   - 显示当前 TUN 设备名称
   - 显示流量统计（入站/出站包数）
   - 显示上游 VPN 信息

2. **上游选择 UI**
   - 下拉菜单选择上游代理应用
   - 显示每个应用的延迟和可用性

3. **错误处理优化**
   - 权限不足时提示用户以管理员身份运行
   - 提供故障排除指南

4. **高级设置**
   - 自定义 TUN IP 地址
   - 自定义 DNS 服务器
   - MTU 设置

5. **路由配置**
   - 自动配置系统路由表
   - DNS 劫持配置

## 文件变更清单

### 新增文件
- `src/components/TunExample.tsx` - TUN API 调用示例
- `docs/TUN_IMPLEMENTATION.md` - TUN 实现文档

### 修改文件
- `src-tauri/Cargo.toml` - 添加 tun 依赖
- `src-tauri/src/tun.rs` - TUN 模块实现
- `src-tauri/src/lib.rs` - 集成 TUN 命令
- `src/store/index.ts` - 添加 TUN 状态和方法
- `src/App.tsx` - 添加 TUN UI 逻辑

## 测试建议

1. **功能测试**
   - [ ] 开启/关闭 TUN 模式
   - [ ] 检查状态同步
   - [ ] 错误场景处理

2. **平台测试**
   - [ ] macOS 测试
   - [ ] Linux 测试
   - [ ] Windows 测试

3. **权限测试**
   - [ ] 普通用户权限
   - [ ] 管理员/root 权限

## 已知限制

1. 需要管理员权限创建 TUN 设备
2. 当前版本只支持 TCP 上游连接
3. DNS 劫持为基础实现（硬编码响应）
4. 需要手动配置系统路由（后续版本自动配置）

## 参考资料

- [TUN crate 文档](https://docs.rs/tun/latest/tun/)
- [Tauri 命令文档](https://tauri.app/develop/calling-commands/)
- [Zustand 状态管理](https://zustand-demo.pmnd.rs/)
