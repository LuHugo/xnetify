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
    }),
    {
      name: 'proxyflow-settings',
      partialize: (state) => ({
        testUrls: state.testUrls,
        timeout: state.timeout,
        holdModeApps: Array.from(state.holdModeApps),
      }),
      merge: (persisted, current) => {
        const persistedState = persisted as { holdModeApps?: string[] } & Partial<AppState>;
        return {
          ...current,
          ...persistedState,
          holdModeApps: new Set(persistedState?.holdModeApps || []),
        };
      },
    }
  )
);