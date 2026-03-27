// src/data/proxies.ts

export interface ProxyDefinition {
  id: string;           // 唯一标识
  name: string;         // 显示名称
  processName: string;  // 进程名称 (用于后端匹配)
  bundleId?: string;    // macOS Bundle ID (用于获取图标/路径)
  defaultPorts: number[]; // 默认端口 (用于快速探测)
  color: string;        // UI 主题色
  icon: string;         // 图标文件名或组件名
  description?: string; // 描述
}

// 默认支持的代理列表
export const DEFAULT_PROXIES: ProxyDefinition[] = [
  {
    id: 'lantern',
    name: 'Lantern',
    processName: 'Lantern',
    bundleId: 'org.getlantern.lantern',
    defaultPorts: [58591, 8787],
    color: '#4caf50',
    icon: 'lantern',
    description: '蓝灯代理'
  },
  {
    id: 'clash',
    name: 'Clash',
    processName: 'clash',
    bundleId: 'com.west2online.ClashX',
    defaultPorts: [7890, 9090],
    color: '#2196f3',
    icon: 'clash',
    description: 'Clash 规则代理'
  },
  {
    id: 'shadowsocks',
    name: 'Shadowsocks',
    processName: 'ShadowsocksX-NG',
    bundleId: 'com.qiuyuzhou.ShadowsocksX-NG',
    defaultPorts: [1080, 6080],
    color: '#ff9800',
    icon: 'shadowsocks',
    description: '影梭代理'
  }
  // 以后想加新的，直接在这里加一行就行！
];