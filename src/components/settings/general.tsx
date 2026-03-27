import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "../ui/button";
import { RefreshCw, Globe, Download, Moon, Sun, Monitor } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
  ItemActions,
  ItemGroup,
} from "../ui/item";

const languageOptions = [
  { value: "en", label: "English" },
  { value: "zh-CN", label: "简体中文" },
];

const themeOptions = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

export default function General({ show = false }: { show?: boolean }) {
  const { theme, setTheme } = useTheme();
  const [language, setLanguage] = useState("en");
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);

  useEffect(() => {
    const savedLang = localStorage.getItem("language") || "en";
    setLanguage(savedLang);
  }, []);

  const handleLanguageChange = (lang: string | null) => {
    if (lang) {
      setLanguage(lang);
      localStorage.setItem("language", lang);
    }
  };

  const handleThemeChange = (newTheme: string | null) => {
    if (newTheme) {
      setTheme(newTheme);
    }
  };

  const checkForUpdate = async () => {
    setCheckingUpdate(true);
    setUpdateStatus(null);
    setTimeout(() => {
      setCheckingUpdate(false);
      setUpdateStatus("已是最新版本");
    }, 2000);
  };

  if (!show) return null;

  return (
    <div className="space-y-4">
      <ItemGroup>
        <Item variant="muted">
          <ItemMedia>
            <Globe className="size-4" />
          </ItemMedia>
          <ItemContent>
            <ItemTitle>Language</ItemTitle>
          </ItemContent>
          <ItemActions>
            <Select value={language} onValueChange={handleLanguageChange}>
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {languageOptions.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </ItemActions>
        </Item>

        <Item variant="muted">
          <ItemMedia>
            {(() => {
              const Icon = themeOptions.find((t) => t.value === theme)?.icon ?? themeOptions[2].icon;
              return <Icon className="size-4" />;
            })()}
          </ItemMedia>
          <ItemContent>
            <ItemTitle>Theme</ItemTitle>
          </ItemContent>
          <ItemActions>
            <Select value={theme || "system"} onValueChange={handleThemeChange}>
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {themeOptions.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </ItemActions>
        </Item>

        <Item variant="muted">
          <ItemMedia>
            <Download className="size-4" />
          </ItemMedia>
          <ItemContent>
            <ItemTitle>Updates</ItemTitle>
          </ItemContent>
          <ItemActions>
            <Button
              variant="outline"
              size="sm"
              onClick={checkForUpdate}
              disabled={checkingUpdate}
              className="flex items-center gap-2"
            >
              <RefreshCw
                className={cn("size-4", checkingUpdate && "animate-spin")}
              />
              Check
            </Button>
          </ItemActions>
        </Item>
      </ItemGroup>

      {updateStatus && (
        <p className="text-sm text-muted-foreground px-4">{updateStatus}</p>
      )}
    </div>
  );
}
