import { useEffect } from "react";
import { toast } from "sonner";
import StatusCard from "./components/StatusCard";
import { Toaster } from "./components/ui/sonner";
import { useAppStore } from "./store";
import NewApp from "./components/NewApp";
import Settings from "./components/settings/index";
import { Switch } from "./components/ui/switch";
import { Item, ItemActions, ItemContent, ItemHeader } from "./components/ui/item";
import { cn } from "./lib/utils";


function App() {
  const {
    apps,
    manualApps,
    lastUpdate,
    shellType,
    setShellType,
    silentTestProxies,
    detectProxies,
    openTerminal,
    proxyEnabled,
    proxyPort,
    toggleProxy,
    checkProxyStatus,
    checkProxyChanges,
  } = useAppStore();

  useEffect(() => {
    const isWindows = navigator.userAgent.includes("Windows");
    setShellType(isWindows ? "powershell" : "bash");
  }, [setShellType]);

  useEffect(() => {
    detectProxies();
    checkProxyStatus();
    const interval = setInterval(() => {
      silentTestProxies();
    }, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [detectProxies, silentTestProxies, checkProxyStatus]);

  useEffect(() => {
    if (!proxyEnabled) return;

    const checkInterval = setInterval(async () => {
      const changes = await checkProxyChanges();
      if (changes.length > 0) {
        const change = changes[0];
        toast.warning(`代理被其他应用修改: ${change.service} -> ${change.changedBy}`);
      }
    }, 30 * 1000);

    return () => clearInterval(checkInterval);
  }, [proxyEnabled, checkProxyChanges]);

  const handleOpenTerminal = async (app: Parameters<typeof openTerminal>[0]) => {
    try {
      await openTerminal(app);
      toast.success(`已为 ${app.name} 打开终端`);
    } catch {
      toast.error("打开终端失败");
    }
  };

  const displayApps = manualApps.length > 0 ? manualApps : apps;

  const handleProxySwitch = async (checked: boolean) => {
    try {
      await toggleProxy(checked);
      toast.success(checked ? "系统代理已开启" : "系统代理已关闭");
    } catch {
      toast.error(checked ? "开启代理失败" : "关闭代理失败");
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <div className="max-w-2xl mx-auto px-4 py-6 space-y-4">
        <div className="flex items-center justify-between">
          <Item variant="muted">
            <ItemContent>
              <ItemHeader>开启代理 (端口 {proxyPort})</ItemHeader>
            </ItemContent>
            <ItemActions>
              <Switch 
                checked={proxyEnabled}
                onCheckedChange={handleProxySwitch} 
                className={cn(proxyEnabled && "data-checked:bg-green-500")}
              />
            </ItemActions>
          </Item>
        </div>
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Proxies</h1>
            <p className="text-sm text-muted-foreground">Auto setup terminal</p>
          </div>
          <div>
            <NewApp />
            <Settings />
          </div>
        </div>

        <StatusCard
          apps={displayApps}
          lastUpdate={lastUpdate}
          onRefresh={detectProxies}
          onOpenTerminal={handleOpenTerminal}
        />

        <div className="text-center text-xs text-muted-foreground/60 pt-2">
          每 5 分钟自动检测 · Xnetify v0.1.0
        </div>
      </div>

      <Toaster />
    </div>
  );
}

export default App;
