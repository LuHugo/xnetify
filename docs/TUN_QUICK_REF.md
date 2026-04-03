# TUN 功能快速参考

## 启用 TUN

```bash
# 确保有可用的代理应用
# 点击 UI 中的 "开启TUN" 开关
```

## 禁用 TUN

```bash
# 点击 UI 中的 "开启TUN" 开关（关闭）
```

## 权限要求

- macOS: `sudo` 或 root
- Linux: root
- Windows: 管理员

## API 调用（JSON 格式）

### 启动

```typescript
const response = await invoke('tun_start', { 
  host: '127.0.0.1', 
  port: 1080 
});
// response: { success: true, data: TunStatus, error: null }
```

### 停止

```typescript
const response = await invoke('tun_stop');
// response: { success: true, data: "TUN 已停止", error: null }
```

### 获取状态

```typescript
const response = await invoke('tun_get_status');
// response: { success: true, data: TunStatus, error: null }
```

## JSON 响应格式

```typescript
// 成功
{
  success: true,
  data: T,        // 类型数据
  error: null
}

// 失败
{
  success: false,
  data: null,
  error: string   // 错误信息
}
```

## 状态字段

```typescript
interface TunStatus {
  active: boolean;           // 是否运行
  config: TunConfig | null; // 配置信息
  packets_in: number;       // 入站包数
  packets_out: number;       // 出站包数
  error: string | null;      // 错误信息
}
```

## 默认配置

- TUN IP: `10.0.0.2`
- 子网掩码: `255.255.255.0`
- DNS: `8.8.8.8`
- MTU: 1500

## 文件位置

- 后端: `src-tauri/src/tun.rs`
- 前端: `src/App.tsx`
- 状态: `src/store/index.ts`
- 文档: `docs/TUN_*.md`
- JSON API: `docs/TUN_JSON_API.md`

## 测试

```bash
# Rust 编译
cd src-tauri && cargo build

# 前端编译
npm run build

# 开发模式
npm run tauri dev
```

## 已知问题

⚠️ 需要管理员权限创建 TUN 设备

## 后续功能

- [ ] 自动路由配置
- [ ] DNS 系统配置
- [ ] 流量统计
- [ ] 多 VPN 支持
