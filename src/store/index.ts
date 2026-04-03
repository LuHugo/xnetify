import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

export interface ProxyApp {
  name: string;
  host: string;
  http_port: number | null;
  socks_port: number | null;
  all_ports: number[];
  pid: number | null;
  tested_http: boolean | null;
  tested_socks: boolean | null;
  tested: boolean | null;
  latency_ms: number | null;
}

export interface TunConfig {
  device_name: string;
  tunnel_ip: string;
  tunnel_netmask: string;
  dns_server: string;
  mtu: number;
  upstream_host: string;
  upstream_port: number;
}

export interface TunStatus {
  active: boolean;
  config: TunConfig | null;
  bytes_in: number;
  bytes_out: number;
  error: string | null;
}

export interface PrivilegeResult {
  success: boolean;
  needs_restart: boolean;
  message: string;
}

export type Mode = 'parent' | 'teen';

export type UrlRuleType = 'block' | 'allow';

export interface TimeSlot {
  start: string;
  end: string;
  days: number[];
}

export interface TimeRule {
  enabled: boolean;
  slots: TimeSlot[];
  max_daily_minutes: number;
}

export interface UrlRule {
  id: string;
  name: string;
  rule_type: UrlRuleType;
  pattern: string;
  category: string | null;
  enabled: boolean;
}

export interface RecommendedSite {
  id: string;
  title: string;
  description: string;
  url: string;
  icon: string;
  category: string;
  enabled: boolean;
}

export interface FilterState {
  mode: Mode;
  is_blocked: boolean;
  remaining_minutes: number;
  used_minutes: number;
  max_minutes: number;
  active_rules: number;
}

const DEFAULT_TEST_URLS = [
  'https://www.google.com',
  'https://huggingface.co',
];

interface AppState {
  apps: ProxyApp[];
  manualApps: ProxyApp[];
  lastUpdate: Date | null;
  shellType: string;
  settingsOpen: boolean;
  testUrls: string[];
  timeout: number;
  prevTestResult: { tested: boolean | null; latency_ms: number | null } | null;
  holdModeApps: Set<string>;
  proxyEnabled: boolean;
  proxyPort: number;
  tunEnabled: boolean;
  tunStatus: TunStatus | null;
  isAdmin: boolean;
  mode: Mode;
  filterState: FilterState;
  urlRules: UrlRule[];
  timeRule: TimeRule;
  recommendedSites: RecommendedSite[];
  isPinVerified: boolean;
  setApps: (apps: ProxyApp[]) => void;
  setManualApps: (apps: ProxyApp[]) => void;
  setLastUpdate: (date: Date | null) => void;
  setShellType: (type: string) => void;
  setSettingsOpen: (open: boolean) => void;
  setTestUrls: (urls: string[]) => void;
  setTimeout: (timeout: number) => void;
  setPrevTestResult: (result: { tested: boolean | null; latency_ms: number | null } | null) => void;
  toggleHoldMode: (appName: string, app: ProxyApp) => Promise<void>;
  updateApp: (name: string, updates: Partial<ProxyApp>) => void;
  detectProxies: () => Promise<void>;
  silentTestProxies: () => Promise<void>;
  testProxies: () => Promise<void>;
  openTerminal: (app: ProxyApp) => Promise<void>;
  toggleProxy: (enabled: boolean) => Promise<void>;
  checkProxyStatus: () => Promise<void>;
  checkProxyChanges: () => Promise<{ service: string; changedBy: string }[]>;
  toggleTun: (enabled: boolean) => Promise<void>;
  checkTunStatus: () => Promise<void>;
  updateTunUpstream: (host: string, port: number) => Promise<void>;
  checkPrivilege: () => Promise<boolean>;
  requestElevation: () => Promise<PrivilegeResult>;
  setMode: (mode: Mode) => Promise<void>;
  refreshFilterState: () => Promise<void>;
  addUrlRule: (rule: UrlRule) => Promise<void>;
  removeUrlRule: (ruleId: string) => Promise<void>;
  updateTimeRule: (rule: TimeRule) => Promise<void>;
  verifyPin: (pin: string) => Promise<boolean>;
  setPin: (pin: string) => Promise<void>;
  checkUrl: (url: string) => Promise<{ allowed: boolean; reason?: string }>;
  loadRecommendedSites: () => Promise<void>;
  addRecommendedSite: (site: RecommendedSite) => Promise<void>;
  removeRecommendedSite: (siteId: string) => Promise<void>;
  updateRecommendedSite: (site: RecommendedSite) => Promise<void>;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      apps: [],
      manualApps: [],
      lastUpdate: null,
      shellType: 'bash',
      settingsOpen: false,
      testUrls: DEFAULT_TEST_URLS,
      timeout: 15,
      prevTestResult: null,
      holdModeApps: new Set<string>(),
      proxyEnabled: false,
      proxyPort: 7890,
      tunEnabled: false,
      tunStatus: null,
      isAdmin: false,
      mode: 'teen',
      filterState: {
        mode: 'teen',
        is_blocked: false,
        remaining_minutes: 480,
        used_minutes: 0,
        max_minutes: 480,
        active_rules: 0,
      },
      urlRules: [],
      timeRule: {
        enabled: false,
        slots: [],
        max_daily_minutes: 480,
      },
      recommendedSites: [],
      isPinVerified: false,

      setApps: (apps) => set({ apps }),
      setManualApps: (manualApps) => set({ manualApps }),
      setLastUpdate: (lastUpdate) => set({ lastUpdate }),
      setShellType: (shellType) => set({ shellType }),
      setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
      setTestUrls: (testUrls) => set({ testUrls }),
      setTimeout: (timeout) => set({ timeout }),
      setPrevTestResult: (prevTestResult) => set({ prevTestResult }),

      toggleHoldMode: async (appName, app) => {
        const { holdModeApps, shellType } = get();
        const newHoldModeApps = new Set(holdModeApps);

        if (newHoldModeApps.has(appName)) {
          newHoldModeApps.delete(appName);
          await invoke('remove_proxy_config', { shellType });
        } else {
          newHoldModeApps.add(appName);
          await invoke('write_proxy_config', { app, shellType });
        }
        set({ holdModeApps: newHoldModeApps });
      },

      updateApp: (name, updates) => set((state) => ({
        apps: state.apps.map((app) =>
          app.name === name ? { ...app, ...updates } : app
        ),
      })),

      silentTestProxies: async () => {
        const { apps, testUrls, timeout, updateApp } = get();
        try {
          const results = await Promise.all(
            apps.map(async (app) => {
              if (!app.all_ports || app.all_ports.length === 0) {
                return { name: app.name, httpPort: null, socksPort: null, latencyMs: null };
              }

              let httpPort: number | null = null;
              let socksPort: number | null = null;
              let latencyMs: number | null = null;

              for (const port of app.all_ports) {
                const httpPromise = invoke<{ success: boolean; latency_ms: number | null }>('test_http_port', {
                  host: app.host,
                  port,
                  timeoutSecs: timeout,
                  testUrls,
                });
                const socksPromise = invoke<{ success: boolean; latency_ms: number | null }>('test_socks_port', {
                  host: app.host,
                  port,
                  timeoutSecs: timeout,
                  testUrls,
                });

                const [httpResult, socksResult] = await Promise.all([httpPromise, socksPromise]);

                if (httpResult.success && !httpPort) {
                  httpPort = port;
                  if (!latencyMs) latencyMs = httpResult.latency_ms;
                }

                if (socksResult.success && !socksPort) {
                  socksPort = port;
                  latencyMs = socksResult.latency_ms;
                }

                if (httpPort && socksPort) break;
              }

              return { name: app.name, httpPort, socksPort, latencyMs };
            })
          );

          results.forEach((result) => {
            const isAvailable = result.httpPort !== null || result.socksPort !== null;
            updateApp(result.name, {
              tested: isAvailable,
              http_port: result.httpPort,
              socks_port: result.socksPort,
              tested_http: result.httpPort !== null,
              tested_socks: result.socksPort !== null,
              latency_ms: result.latencyMs,
            });
          });
        } catch (error) {
          console.error('Silent test failed:', error);
        }
      },

      testProxies: async () => {
        const { apps, testUrls, timeout, updateApp } = get();
        try {
          for (const app of apps) {
            if (!app.all_ports || app.all_ports.length === 0) continue;

            let httpPort: number | null = null;
            let socksPort: number | null = null;
            let latencyMs: number | null = null;

            for (const port of app.all_ports) {
              const httpPromise = invoke<{ success: boolean; latency_ms: number | null }>('test_http_port', {
                host: app.host,
                port,
                timeoutSecs: timeout,
                testUrls,
              });
              const socksPromise = invoke<{ success: boolean; latency_ms: number | null }>('test_socks_port', {
                host: app.host,
                port,
                timeoutSecs: timeout,
                testUrls,
              });

              const [httpResult, socksResult] = await Promise.allSettled([httpPromise, socksPromise]);

              if (httpResult.status === 'rejected' || socksResult.status === 'rejected') {
                console.error(`Failed to test port ${port}`);
                continue;
              }

              if (httpResult.value.success && !httpPort) {
                httpPort = port;
                if (!latencyMs) latencyMs = httpResult.value.latency_ms;
              }

              if (socksResult.value.success && !socksPort) {
                socksPort = port;
                latencyMs = socksResult.value.latency_ms;
              }

              if (httpPort && socksPort) break;
            }

            const isAvailable = httpPort !== null || socksPort !== null;
            updateApp(app.name, {
              tested: isAvailable,
              http_port: httpPort,
              socks_port: socksPort,
              tested_http: httpPort !== null,
              tested_socks: socksPort !== null,
              latency_ms: latencyMs,
            });
          }
        } catch (error) {
          console.error('Failed to test proxies:', error);
        }
      },

      detectProxies: async () => {
        const { testProxies, prevTestResult, setApps, setManualApps, setLastUpdate, setPrevTestResult } = get();
        try {
          const detected = await invoke<ProxyApp[]>('detect_proxy_ports');

          if (detected.length > 0) {
            setManualApps([]);
            const appsWithNullPorts = detected.map((app) => ({
              ...app,
              http_port: null,
              socks_port: null,
              tested_http: null as boolean | null,
              tested_socks: null as boolean | null,
              tested: null as boolean | null,
              latency_ms: null,
            }));
            setApps(appsWithNullPorts);
            testProxies();
          } else {
            if (prevTestResult && prevTestResult.tested !== null) {
              setPrevTestResult(null);
              setApps([]);
            }
          }
          setLastUpdate(new Date());
        } catch (error) {
          console.error('Failed to detect proxies:', error);
        }
      },

      openTerminal: async (app) => {
        const { shellType } = get();
        try {
          const cmd = await invoke<{ set_command: string }>('generate_proxy_command', {
            apps: [app],
            shellType,
          });
          await invoke('open_new_terminal', { command: cmd.set_command });
        } catch (error) {
          console.error('Failed to open terminal:', error);
          throw error;
        }
      },

      toggleProxy: async (enabled) => {
        const { proxyPort } = get();
        try {
          if (enabled) {
            await invoke('set_system_proxy', { port: proxyPort });
          } else {
            await invoke('unset_system_proxy');
          }
          set({ proxyEnabled: enabled });
        } catch (error) {
          console.error('Failed to toggle proxy:', error);
          throw error;
        }
      },

      checkProxyStatus: async () => {
        try {
          const enabled = await invoke<boolean>('is_proxy_enabled');
          set({ proxyEnabled: enabled });
        } catch (error) {
          console.error('Failed to check proxy status:', error);
        }
      },

      checkProxyChanges: async () => {
        const { proxyPort } = get();
        try {
          const changes = await invoke<{ service: string; changed_by: string }[]>('check_proxy_changes', { port: proxyPort });
          return changes.map(c => ({ service: c.service, changedBy: c.changed_by }));
        } catch (error) {
          console.error('Failed to check proxy changes:', error);
          return [];
        }
      },

      toggleTun: async (enabled) => {
        const { apps } = get();
        try {
          if (enabled) {
            const upstreamApp = apps.find(app => app.tested);
            if (!upstreamApp) {
              throw new Error('没有可用的代理应用');
            }

            const upstreamHost = upstreamApp.host;
            const upstreamPort = upstreamApp.socks_port || upstreamApp.http_port || 1080;

            const response = await invoke<{ success: boolean; data: TunStatus | null; error: string | null }>('tun_start', { host: upstreamHost, port: upstreamPort });
            if (!response.success || !response.data) {
              throw new Error(response.error || '启动失败');
            }
            set({ tunEnabled: true, tunStatus: response.data });
          } else {
            const response = await invoke<{ success: boolean; data: string | null; error: string | null }>('tun_stop');
            if (!response.success) {
              throw new Error(response.error || '停止失败');
            }
            set({ tunEnabled: false, tunStatus: null });
          }
        } catch (error) {
          console.error('Failed to toggle TUN:', error);
          throw error;
        }
      },

      checkTunStatus: async () => {
        try {
          const response = await invoke<{ success: boolean; data: TunStatus | null; error: string | null }>('tun_get_status');
          if (response.success && response.data) {
            set({ 
              tunEnabled: response.data.active, 
              tunStatus: response.data 
            });
          }
        } catch (error) {
          console.error('Failed to check TUN status:', error);
        }
      },

      updateTunUpstream: async (host, port) => {
        try {
          const response = await invoke<{ success: boolean; data: string | null; error: string | null }>('tun_update_upstream', { host, port });
          if (!response.success) {
            throw new Error(response.error || '更新失败');
          }
          await get().checkTunStatus();
        } catch (error) {
          console.error('Failed to update TUN upstream:', error);
          throw error;
        }
      },

      checkPrivilege: async () => {
        try {
          const response = await invoke<PrivilegeResult>('check_admin');
          set({ isAdmin: response.success });
          return response.success;
        } catch (error) {
          console.error('Failed to check privilege:', error);
          set({ isAdmin: false });
          return false;
        }
      },

      requestElevation: async () => {
        try {
          const response = await invoke<PrivilegeResult>('request_elevation');
          return response;
        } catch (error) {
          console.error('Failed to request elevation:', error);
          return {
            success: false,
            needs_restart: false,
            message: error instanceof Error ? error.message : '提权请求失败'
          };
        }
      },

      setMode: async (mode) => {
        try {
          await invoke('set_mode', { mode });
          set({ mode });
          await get().refreshFilterState();
        } catch (error) {
          console.error('Failed to set mode:', error);
          throw error;
        }
      },

      refreshFilterState: async () => {
        try {
          const state = await invoke<FilterState>('get_filter_state');
          set({ filterState: state });
        } catch (error) {
          console.error('Failed to refresh filter state:', error);
        }
      },

      addUrlRule: async (rule) => {
        try {
          await invoke('add_url_rule', { rule });
          set((state) => ({ urlRules: [...state.urlRules, rule] }));
          await get().refreshFilterState();
        } catch (error) {
          console.error('Failed to add URL rule:', error);
          throw error;
        }
      },

      removeUrlRule: async (ruleId) => {
        try {
          await invoke('remove_url_rule', { ruleId });
          set((state) => ({ urlRules: state.urlRules.filter(r => r.id !== ruleId) }));
          await get().refreshFilterState();
        } catch (error) {
          console.error('Failed to remove URL rule:', error);
          throw error;
        }
      },

      updateTimeRule: async (rule) => {
        try {
          await invoke('set_time_rule', { rule });
          set({ timeRule: rule });
          await get().refreshFilterState();
        } catch (error) {
          console.error('Failed to update time rule:', error);
          throw error;
        }
      },

      verifyPin: async (pin) => {
        try {
          const valid = await invoke<boolean>('verify_pin', { pin });
          set({ isPinVerified: valid });
          return valid;
        } catch (error) {
          console.error('Failed to verify PIN:', error);
          return false;
        }
      },

      setPin: async (pin) => {
        try {
          await invoke('set_pin', { pin });
        } catch (error) {
          console.error('Failed to set PIN:', error);
          throw error;
        }
      },

      checkUrl: async (url) => {
        try {
          const [allowed, reason] = await invoke<[boolean, string | null]>('check_url', { url });
          return { allowed, reason: reason ?? undefined };
        } catch (error) {
          console.error('Failed to check URL:', error);
          return { allowed: true };
        }
      },

      loadRecommendedSites: async () => {
        try {
          const sites = await invoke<RecommendedSite[]>('get_recommended_sites');
          set({ recommendedSites: sites });
        } catch (error) {
          console.error('Failed to load recommended sites:', error);
        }
      },

      addRecommendedSite: async (site: RecommendedSite) => {
        try {
          await invoke('add_recommended_site', { site });
          set((state) => ({ recommendedSites: [...state.recommendedSites, site] }));
        } catch (error) {
          console.error('Failed to add recommended site:', error);
          throw error;
        }
      },

      removeRecommendedSite: async (siteId: string) => {
        try {
          await invoke('remove_recommended_site', { siteId });
          set((state) => ({ recommendedSites: state.recommendedSites.filter(s => s.id !== siteId) }));
        } catch (error) {
          console.error('Failed to remove recommended site:', error);
          throw error;
        }
      },

      updateRecommendedSite: async (site: RecommendedSite) => {
        try {
          await invoke('update_recommended_site', { site });
          set((state) => ({
            recommendedSites: state.recommendedSites.map(s => s.id === site.id ? site : s)
          }));
        } catch (error) {
          console.error('Failed to update recommended site:', error);
          throw error;
        }
      },
    }),
    {
      name: 'xnetify-settings',
      partialize: (state) => ({
        testUrls: state.testUrls,
        timeout: state.timeout,
        holdModeApps: Array.from(state.holdModeApps),
        mode: state.mode,
        urlRules: state.urlRules,
        timeRule: state.timeRule,
      }),
      merge: (persisted, current) => {
        const persistedState = persisted as { holdModeApps?: string[]; mode?: Mode; urlRules?: UrlRule[]; timeRule?: TimeRule } & Partial<AppState>;
        return {
          ...current,
          ...persistedState,
          holdModeApps: new Set(persistedState?.holdModeApps || []),
        };
      },
    }
  )
);
