import { useEffect } from "react";
import { toast } from "sonner";
import { Toaster } from "@/components/ui/sonner";
import { useAppStore } from "./store";
import { Shield, Lock, Unlock, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ThemeToggle } from "@/components/ThemeToggle";
import { ModeSwitch } from "./components/ModeSwitch";
import { TimeRules } from "./components/TimeRules";
import { UrlRules } from "./components/UrlRules";
import { RecommendedSites } from "./components/RecommendedSites";
import { TeenDashboard } from "./components/TeenDashboard";
import { TunToggle } from "./components/TunToggle";

function App() {
  const {
    proxyEnabled,
    toggleProxy,
    checkProxyStatus,
    mode,
    setMode,
  } = useAppStore();

  useEffect(() => {
    checkProxyStatus();
  }, [checkProxyStatus]);

  const handleProxySwitch = async (checked: boolean) => {
    try {
      await toggleProxy(checked);
      toast.success(checked ? "守护已开启" : "守护已关闭");
    } catch {
      toast.error("操作失败");
    }
  };

  const handleSwitchToTeen = async () => {
    await setMode('teen');
    toast.success("已切换到成长模式");
  };

  const isProtected = mode === 'teen';

  return (
    <div className="min-h-screen bg-background">
      {isProtected ? (
        <TeenDashboard />
      ) : (
        <>
          <header className="flex items-center justify-between px-6 py-4 border-b">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-xl bg-primary/10">
                <Shield className="h-6 w-6 text-primary" />
              </div>
              <span className="font-medium">成长守护</span>
            </div>
            <div className="flex items-center gap-4">
              <ThemeToggle />
              <Button
                variant="outline"
                size="sm"
                onClick={handleSwitchToTeen}
                className="gap-2"
              >
                <Sun className="h-4 w-4" />
                返回成长模式
              </Button>
              <div className="flex items-center gap-2">
                <div className={`w-2 h-2 rounded-full ${proxyEnabled ? 'bg-emerald-400 animate-pulse' : 'bg-muted'}`} />
                <span className={`text-sm ${proxyEnabled ? 'text-emerald-400' : 'text-muted-foreground'}`}>
                  {proxyEnabled ? '守护中' : '未启用'}
                </span>
              </div>
            </div>
          </header>

          <main className="max-w-5xl mx-auto px-6 py-8">
            <Card className="mb-6">
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="p-3 rounded-xl bg-primary/10">
                      {proxyEnabled ? <Lock className="h-6 w-6 text-primary" /> : <Unlock className="h-6 w-6 text-muted-foreground" />}
                    </div>
                    <div>
                      <p className="text-lg font-medium">家长守护中心</p>
                      <p className="text-sm text-muted-foreground">管理陪伴设置</p>
                    </div>
                  </div>
                  <Button
                    onClick={() => handleProxySwitch(!proxyEnabled)}
                    className={proxyEnabled ? '' : 'bg-muted hover:bg-muted/80'}
                  >
                    {proxyEnabled ? '守护已开启' : '开启守护'}
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Tabs defaultValue="mode" className="w-full">
              <TabsList className="grid w-full grid-cols-5">
                <TabsTrigger value="mode">守护模式</TabsTrigger>
                <TabsTrigger value="sites">推荐内容</TabsTrigger>
                <TabsTrigger value="time">时间管理</TabsTrigger>
                <TabsTrigger value="rules">网址规则</TabsTrigger>
                <TabsTrigger value="tun">网络模式</TabsTrigger>
              </TabsList>
              <TabsContent value="mode" className="mt-6">
                <ModeSwitch />
              </TabsContent>
              <TabsContent value="sites" className="mt-6">
                <RecommendedSites />
              </TabsContent>
              <TabsContent value="time" className="mt-6">
                <TimeRules />
              </TabsContent>
              <TabsContent value="rules" className="mt-6">
                <UrlRules />
              </TabsContent>
              <TabsContent value="tun" className="mt-6">
                <TunToggle />
              </TabsContent>
            </Tabs>
          </main>

          <footer className="text-center py-4 text-sm text-muted-foreground">
            成长守护
          </footer>
        </>
      )}

      <Toaster />
    </div>
  );
}

export default App;
