import { useState, useEffect } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Network, Shield, AlertCircle, CheckCircle2, Loader2, KeyRound, RefreshCw } from 'lucide-react';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

interface TunStatus {
  active: boolean;
  config: {
    device_name: string;
    tunnel_ip: string;
    tunnel_netmask: string;
  } | null;
  bytes_in: number;
  bytes_out: number;
  error: string | null;
}

interface TunResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

interface PrivilegeResult {
  success: boolean;
  needs_restart: boolean;
  message: string;
}

export function TunToggle() {
  const { tunEnabled, tunStatus, checkTunStatus } = useAppStore();
  const [isLoading, setIsLoading] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);
  const [checkingPrivilege, setCheckingPrivilege] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const checkPrivilege = async () => {
    setCheckingPrivilege(true);
    try {
      const result = await invoke<PrivilegeResult>('check_admin');
      setIsAdmin(result.success);
    } catch (error) {
      console.error('Failed to check privilege:', error);
      setIsAdmin(false);
    } finally {
      setCheckingPrivilege(false);
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await checkTunStatus();
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    checkPrivilege();
    checkTunStatus();
    
    const interval = setInterval(() => {
      checkTunStatus();
    }, 5000);
    
    return () => clearInterval(interval);
  }, []);

  const handleRequestElevation = async () => {
    try {
      const result = await invoke<PrivilegeResult>('request_elevation');
      if (result.success && result.needs_restart) {
        toast.success('权限获取成功，请重启应用');
      } else if (result.success) {
        toast.success('已拥有管理员权限');
        setIsAdmin(true);
      } else {
        toast.error(result.message);
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleToggle = async () => {
    if (!isAdmin) {
      toast.error('需要管理员权限才能开启 TUN 模式');
      return;
    }

    setIsLoading(true);
    try {
      if (tunEnabled) {
        const response = await invoke<TunResponse<string>>('tun_stop');
        if (!response.success) {
          toast.error(response.error || '停止失败');
          return;
        }
        toast.success('TUN 模式已关闭');
        await checkTunStatus();
      } else {
        const response = await invoke<TunResponse<TunStatus>>('tun_start', { 
          host: '127.0.0.1', 
          port: 7890 
        });
        if (!response.success || !response.data) {
          toast.error(response.error || '启动失败');
          return;
        }
        toast.success('TUN 模式已开启');
        await checkTunStatus();
      }
    } catch (error) {
      console.error('TUN toggle failed:', error);
      toast.error(String(error));
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Network className="h-5 w-5 text-blue-400" />
              TUN 模式
            </CardTitle>
            <CardDescription>
              接管全部网络流量，实现更全面的守护
            </CardDescription>
          </div>
          <Button 
            variant="ghost" 
            size="icon"
            onClick={handleRefresh}
            disabled={refreshing}
          >
            <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {!isAdmin && !checkingPrivilege && (
            <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20">
              <div className="flex items-start gap-3">
                <AlertCircle className="h-5 w-5 text-amber-400 mt-0.5" />
                <div className="flex-1">
                  <p className="font-medium text-amber-400 mb-1">需要管理员权限</p>
                  <p className="text-sm text-muted-foreground mb-3">
                    TUN 模式需要管理员权限来创建虚拟网卡
                  </p>
                  <Button 
                    onClick={handleRequestElevation}
                    className="bg-amber-500 hover:bg-amber-600"
                    size="sm"
                  >
                    <KeyRound className="h-4 w-4 mr-2" />
                    获取管理员权限
                  </Button>
                </div>
              </div>
            </div>
          )}

          <div className="flex items-center justify-between p-4 rounded-xl bg-muted/50">
            <div className="flex items-center gap-3">
              <div className={`p-2 rounded-lg ${tunEnabled ? 'bg-blue-500/20' : 'bg-slate-700/50'}`}>
                {tunEnabled ? (
                  <Shield className="h-5 w-5 text-blue-400" />
                ) : (
                  <Shield className="h-5 w-5 text-slate-500" />
                )}
              </div>
              <div>
                <p className="font-medium">
                  {tunEnabled ? 'TUN 模式已开启' : 'TUN 模式已关闭'}
                </p>
                <p className="text-sm text-muted-foreground">
                  {tunEnabled ? '正在守护所有网络流量' : '使用系统代理模式'}
                </p>
              </div>
            </div>
            <Button
              variant={tunEnabled ? "destructive" : "default"}
              onClick={handleToggle}
              disabled={isLoading || checkingPrivilege || !isAdmin}
              className={!tunEnabled && isAdmin ? 'bg-blue-500 hover:bg-blue-600' : ''}
            >
              {isLoading || checkingPrivilege ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : tunEnabled ? '关闭' : '开启'}
            </Button>
          </div>

          <div className="p-4 rounded-xl bg-muted/30 space-y-3">
            <div className="flex items-center gap-2">
              {tunStatus?.active ? (
                <CheckCircle2 className="h-4 w-4 text-emerald-400" />
              ) : (
                <AlertCircle className="h-4 w-4 text-slate-500" />
              )}
              <span className="font-medium text-sm">状态监控</span>
            </div>
            
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div className="p-3 rounded-lg bg-background/50">
                <div className="text-muted-foreground mb-1">运行状态</div>
                <div className={`font-medium ${tunStatus?.active ? 'text-emerald-400' : 'text-slate-500'}`}>
                  {tunStatus?.active ? '运行中' : '已停止'}
                </div>
              </div>
              
              <div className="p-3 rounded-lg bg-background/50">
                <div className="text-muted-foreground mb-1">设备名称</div>
                <div className="font-medium truncate">
                  {tunStatus?.config?.device_name || '-'}
                </div>
              </div>
              
              <div className="p-3 rounded-lg bg-background/50">
                <div className="text-muted-foreground mb-1">TUN IP</div>
                <div className="font-medium">
                  {tunStatus?.config?.tunnel_ip || '-'}
                </div>
              </div>
              
              <div className="p-3 rounded-lg bg-background/50">
                <div className="text-muted-foreground mb-1">流量统计</div>
                <div className="font-medium">
                  {formatBytes(tunStatus?.bytes_in || 0)} ↓ / {formatBytes(tunStatus?.bytes_out || 0)} ↑
                </div>
              </div>
            </div>

            {tunStatus?.error && (
              <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
                <div className="text-red-400 text-sm font-medium mb-1">错误信息</div>
                <div className="text-red-300 text-sm">{tunStatus.error}</div>
              </div>
            )}
          </div>

          <div className="text-xs text-muted-foreground p-3 rounded-lg bg-slate-500/10 border border-slate-500/20">
            <p className="font-medium text-slate-400 mb-1">工作原理</p>
            <p>TUN 模式会创建一个虚拟网卡，将所有网络流量通过应用转发，实现完整的流量守护。需要管理员权限。</p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

import { useAppStore } from '../store';
