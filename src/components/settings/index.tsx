import { useState } from "react";
import { Button } from "../ui/button";
import { Dialog, DialogContent, DialogTrigger } from "../ui/dialog";
import Sidebar from "./sidebar";
import { useSettingsStore } from "@/store/settings";
import { SettingsIcon } from "lucide-react";
import General from "./general";
import Proxies from "./proxies";
import Apps from "./apps";
import Connection from "./connection";

export default function Settings() {
    const { activeItem, setActiveItem, setOpen, open } = useSettingsStore();
    return (
        <Dialog defaultOpen={open} onOpenChange={(open) => !open && setOpen(false)}>
            <DialogTrigger render={<Button
                variant="ghost"
                size="icon"
                title="设置"
            >
                <SettingsIcon className="size-4" />
            </Button>} />
            <DialogContent className="flex p-0 space-x-0 gap-0 max-w-[600px]! min-h-[400px] max-h-screen" showCloseButton={false}>
                <Sidebar />
                <div className="p-3 w-full felx-1">
                    <General show={activeItem == "general"} />
                    <Proxies show={activeItem == "proxies"} />
                    <Apps show={activeItem == "apps"} />
                    <Connection show={activeItem == "connection"} />
                </div>
            </DialogContent>
        </Dialog>
    );
}