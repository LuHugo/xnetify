# TUN API JSON 响应格式

## 概述

所有 TUN 相关命令现在都返回统一的 JSON 格式响应。

## 响应结构

### 成功响应

```typescript
{
  success: true,
  data: T,        // 实际数据
  error: null
}
```

### 错误响应

```typescript
{
  success: false,
  data: null,
  error: string   // 错误信息
}
```

## API 命令

### 1. tun_start

**请求**
```typescript
await invoke('tun_start', { 
  host: string, 
  port: number 
});
```

**响应 (成功)**
```typescript
{
  success: true,
  data: {
    active: true,
    config: {
      device_name: "utun3",
      tunnel_ip: "10.0.0.2",
      tunnel_netmask: "255.255.255.0",
      dns_server: "8.8.8.8",
      mtu: 1500,
      upstream_host: "127.0.0.1",
      upstream_port: 1080
    },
    packets_in: 0,
    packets_out: 0,
    error: null
  },
  error: null
}
```

**响应 (失败)**
```typescript
{
  success: false,
  data: null,
  error: "创建设备失败: Permission denied"
}
```

### 2. tun_stop

**请求**
```typescript
await invoke('tun_stop');
```

**响应 (成功)**
```typescript
{
  success: true,
  data: "TUN 已停止",
  error: null
}
```

**响应 (失败)**
```typescript
{
  success: false,
  data: null,
  error: "TUN 设备未运行"
}
```

### 3. tun_get_status

**请求**
```typescript
await invoke('tun_get_status');
```

**响应 (成功)**
```typescript
{
  success: true,
  data: {
    active: true,
    config: {
      device_name: "utun3",
      tunnel_ip: "10.0.0.2",
      tunnel_netmask: "255.255.255.0",
      dns_server: "8.8.8.8",
      mtu: 1500,
      upstream_host: "127.0.0.1",
      upstream_port: 1080
    },
    packets_in: 1234,
    packets_out: 5678,
    error: null
  },
  error: null
}
```

### 4. tun_update_upstream

**请求**
```typescript
await invoke('tun_update_upstream', { 
  host: string, 
  port: number 
});
```

**响应 (成功)**
```typescript
{
  success: true,
  data: "上游已更新",
  error: null
}
```

**响应 (失败)**
```typescript
{
  success: false,
  data: null,
  error: "TUN 设备未运行"
}
```

## 前端调用示例

### 启动 TUN

```typescript
async function startTun(host: string, port: number) {
  const response = await invoke<{
    success: boolean;
    data: TunStatus | null;
    error: string | null;
  }>('tun_start', { host, port });

  if (response.success && response.data) {
    console.log('TUN 启动成功:', response.data);
  } else {
    console.error('TUN 启动失败:', response.error);
  }
}
```

### 停止 TUN

```typescript
async function stopTun() {
  const response = await invoke<{
    success: boolean;
    data: string | null;
    error: string | null;
  }>('tun_stop');

  if (response.success) {
    console.log('TUN 停止成功:', response.data);
  } else {
    console.error('TUN 停止失败:', response.error);
  }
}
```

### 获取状态

```typescript
async function getStatus() {
  const response = await invoke<{
    success: boolean;
    data: TunStatus | null;
    error: string | null;
  }>('tun_get_status');

  if (response.success && response.data) {
    console.log('状态:', response.data);
  } else {
    console.error('获取状态失败:', response.error);
  }
}
```

## 状态管理中的使用

在 Zustand store 中已封装好了处理逻辑：

```typescript
// 切换 TUN
await toggleTun(true);  // 自动处理响应

// 检查状态
await checkTunStatus(); // 自动更新 store

// 更新上游
await updateTunUpstream('vpn.com', 443); // 自动处理响应
```

## 错误处理

所有错误都会通过 toast 显示给用户：

```typescript
try {
  await toggleTun(true);
  toast.success('TUN 模式已开启');
} catch (error) {
  const errorMessage = error instanceof Error ? error.message : '未知错误';
  toast.error(`开启 TUN 失败: ${errorMessage}`);
}
```

## 类型定义

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

interface TunResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}
```

## 优势

1. **统一的响应格式** - 所有命令返回相同的结构
2. **清晰的错误信息** - 错误详情在 `error` 字段
3. **类型安全** - TypeScript 完整类型支持
4. **易于调试** - 响应用于日志和调试
5. **易于扩展** - 可以轻松添加新字段

## 迁移指南

如果之前直接调用 Tauri 命令，需要更新处理逻辑：

### 旧代码
```typescript
const status = await invoke<TunStatus>('tun_get_status');
console.log(status.active); // 直接访问
```

### 新代码
```typescript
const response = await invoke<{
  success: boolean;
  data: TunStatus | null;
  error: string | null;
}>('tun_get_status');

if (response.success && response.data) {
  console.log(response.data.active); // 通过 data 字段访问
} else {
  console.error(response.error);
}
```

推荐使用封装好的 store 方法，它们已经处理了响应格式：

```typescript
// 推荐：使用 store 方法
await checkTunStatus();

// 不推荐：直接调用
// const response = await invoke(...);
// if (response.success && response.data) { ... }
```
