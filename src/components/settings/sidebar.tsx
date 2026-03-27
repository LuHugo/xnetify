import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/store/settings";
import { SettingsSidebarItem } from "@/types";
import { Globe2Icon, LayoutGridIcon, Settings2Icon, ShieldCheck } from "lucide-react";
import { useState } from "react";

const sidebarItems = [
    {
        id: "general",
        name: "General",
        icon: Settings2Icon,
    },
    {
        id: "proxies",
        name: "Proxies",
        icon: ShieldCheck,
    },
    {
        id: "apps",
        name: "Apps",
        icon: LayoutGridIcon,
    },
    {
        id: "connection",
        name: "Connection",
        icon: Globe2Icon,
    }
]
export default function Sidebar() {
    const { activeItem, setActiveItem } = useSettingsStore();
    return (
        <div className="flex flex-col bg-accent/30 border-r rounded-l-xl min-w-40 justify-between">
            <div>
                <div className="p-4">
                    <h2 className="text-md font-semibold">Settings</h2>
                </div>
                <ul className="px-4 justify-center items-center align-center space-y-1">
                    {
                        sidebarItems.map((item) => (
                            <li key={item.id} onClick={() => setActiveItem(item.id as SettingsSidebarItem)} className={cn("flex gap-2 px-2 py-1.5 rounded-md items-center cursor-default", activeItem === item.id ? "bg-accent" : "")}>
                                <item.icon className="size-4" />
                                {item.name}
                            </li>
                        ))
                    }
                </ul>
            </div>
            <p className="text-center text-xs text-muted-foreground mt-4 mb-4">
                Xnetify &copy; 2026
                <br />
                version: 0.0.1
            </p>
        </div>
    );
}