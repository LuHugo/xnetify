import { FC, useState, useEffect } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Trash2, RotateCcw, Plus, Settings2Icon, ShieldCheck, LayoutGridIcon, Globe2Icon } from "lucide-react";
import { useAppStore } from "@/store";

const DEFAULT_TEST_URLS = [
  "https://www.google.com",
  "https://huggingface.co",
];

const Settings: FC = () => {
  const {
    settingsOpen,
    setSettingsOpen,
    testUrls,
    setTestUrls,
    timeout,
    setTimeout,
  } = useAppStore();

  const [localUrls, setLocalUrls] = useState<string[]>(testUrls);
  const [localTimeout, setLocalTimeout] = useState(timeout);
  const [newUrl, setNewUrl] = useState("");

  useEffect(() => {
    setLocalUrls(testUrls);
    setLocalTimeout(timeout);
  }, [testUrls, timeout]);

  const handleAddUrl = () => {
    const trimmed = newUrl.trim();
    if (trimmed && !localUrls.includes(trimmed)) {
      const updated = [...localUrls, trimmed];
      setLocalUrls(updated);
      setTestUrls(updated);
    }
    setNewUrl("");
  };

  const handleRemoveUrl = (url: string) => {
    const updated = localUrls.filter((u) => u !== url);
    setLocalUrls(updated);
    setTestUrls(updated);
  };

  const handleTimeoutChange = (value: number | readonly number[]) => {
    const numValue = Array.isArray(value) ? value[0] : value;
    setLocalTimeout(numValue);
    setTimeout(numValue);
  };

  const handleReset = () => {
    setLocalUrls(DEFAULT_TEST_URLS);
    setTestUrls(DEFAULT_TEST_URLS);
  };

  return (
    <Dialog open={settingsOpen} onOpenChange={(open) => !open && setSettingsOpen(false)}>
      <DialogContent className="flex p-0">
        <div className="sidebar bg-accent/30 border-r rounded-l-xl min-w-40">
          <div className="p-4">
            <h2 className="text-md font-semibold">Settings</h2>
          </div>
          <ul className="px-4 justify-center items-center align-center space-y-1">
            <li className="flex gap-2 bg-accent px-2 py-1.5 rounded-md items-center cursor-pointer">
              <Settings2Icon className="size-4" />
              General
            </li>
            <li className="flex gap-2 items-center px-2 py-1.5">
              <ShieldCheck className="size-4" />
              Proxies
            </li>
            <li className="flex gap-2 items-center px-2 py-1.5">
              <LayoutGridIcon className="size-4" />
              Apps
            </li>
            <li className="flex gap-2 items-center px-2 py-1.5">
              <Globe2Icon className="size-4" />
              Connection
            </li>
          </ul>
        </div>
        <div className="space-y-6 py-4">
          <div className="space-y-3">
            <label className="text-sm font-medium">测试网址</label>
            <div className="space-y-2">
              {localUrls.map((url, index) => (
                <div key={index} className="flex items-center gap-2">
                  <span className="flex-1 text-sm text-muted-foreground bg-muted px-3 py-2 rounded-md truncate">
                    {url}
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemoveUrl(url)}
                    className="text-destructive hover:text-destructive hover:bg-destructive/10"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
            <div className="flex gap-2">
              <Input
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleAddUrl()}
                placeholder="输入新的测试网址..."
              />
              <Button onClick={handleAddUrl}>
                <Plus className="h-4 w-4 mr-1" />
                添加
              </Button>
            </div>
          </div>

          <div className="space-y-3">
            <label className="text-sm font-medium">
              超时时间: {localTimeout} 秒
            </label>
            <Slider
              value={localTimeout}
              onValueChange={handleTimeoutChange}
              min={5}
              max={60}
              step={5}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>5s</span>
              <span>60s</span>
            </div>
          </div>
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="ghost" onClick={handleReset}>
            <RotateCcw className="h-4 w-4 mr-1" />
            重置为默认
          </Button>
          <Button onClick={() => setSettingsOpen(false)}>完成</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default Settings;
