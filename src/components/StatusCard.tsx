import { FC } from "react";
import { Card, CardContent, CardFooter, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Terminal, CheckCircle2, XCircle, AlertCircle } from "lucide-react";
import { Switch } from "./ui/switch";
import { Label } from "./ui/label";
import ProxyPortCard from "./ProxyPortCard";
import { useAppStore, type ProxyApp } from "@/store";

interface StatusCardProps {
  apps: ProxyApp[];
  lastUpdate: Date | null;
  onRefresh: () => void;
  onOpenTerminal: (app: ProxyApp) => void;
}

const StatusCard: FC<StatusCardProps> = ({ apps, lastUpdate, onRefresh, onOpenTerminal }) => {
  const { holdModeApps, toggleHoldMode } = useAppStore();

  return (
    <>
      {apps.length === 0 ? (
        <div className="text-center py-8">
          <div className="mx-auto mb-3 rounded-full bg-muted w-12 h-12 flex items-center justify-center">
            <AlertCircle className="h-6 w-6 text-muted-foreground" />
          </div>
          <p className="text-muted-foreground text-sm">未检测到代理软件</p>
          <p className="text-muted-foreground/60 text-xs mt-1">请确保代理软件已启动</p>
        </div>
      ) : (
        <div className="space-y-3">
          {apps.map((app, index) => {
            const isHoldMode = holdModeApps.has(app.name);
            return (
              <Card key={`${app.name}-${index}`}>
                <CardHeader className="flex gap-2 flex-row items-center space-y-0">
                  <div className={`size-10 rounded-lg flex items-center justify-center flex-shrink-0 ${
                    app.tested === true
                      ? "bg-green-100 text-green-700"
                      : app.tested === false
                        ? "bg-red-100 text-red-700"
                        : "bg-blue-100 text-blue-700"
                    }`}>
                    {app.tested === true ? (
                      <CheckCircle2 className="w-5 h-5" />
                    ) : app.tested === false ? (
                      <XCircle className="w-5 h-5" />
                    ) : (
                      <span className="text-sm font-semibold">
                        {app.name.charAt(0).toUpperCase()}
                      </span>
                    )}
                  </div>
                  <div className="flex flex-1 items-center gap-2">
                    <div className="flex-1">
                      <h3 className="font-bold truncate">{app.name}</h3>
                      <p className="text-xs text-muted-foreground truncate">{app.host}</p>
                    </div>
                    <Badge>
                      {app.tested === true ? "Healthy" : app.tested === false ? "Unhealthy" : "Pending"}
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent className="grid grid-cols-2 gap-4">
                  <ProxyPortCard host={app.host} portType="http" loading={app.tested_http !== true} port={app.http_port || 0} />
                  <ProxyPortCard host={app.host} portType="socks" loading={app.tested_socks !== true} port={app.socks_port || 0} />
                </CardContent>
                <CardFooter className="flex gap-6">
                  <div className="flex gap-2">
                    <Switch
                      id={`hold-mode-${app.name}`}
                      checked={isHoldMode}
                      onCheckedChange={() => toggleHoldMode(app.name, app)}
                    />
                    <Label htmlFor={`hold-mode-${app.name}`}>Hold Mode</Label>
                  </div>
                  <div>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => onOpenTerminal(app)}
                      className="w-full"
                    >
                      <Terminal className="h-3.5 w-3.5 mr-1" />
                      Open Terminal
                    </Button>
                  </div>
                </CardFooter>
              </Card>
            );
          })}
        </div>
      )}
    </>
  );
};

export default StatusCard;
