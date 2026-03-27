import { CheckCircle2, Copy, LoaderPinwheel, XCircle } from "lucide-react";
import { Button } from "./ui/button";
import { Item, ItemActions, ItemContent, ItemDescription, ItemMedia, ItemTitle } from "./ui/item";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export default function ProxyPortCard({ host, port, loading = true, portType }: { host: string, port: number, loading: boolean, portType: string }) {
    const copyHandler = () => {
        navigator.clipboard.writeText(`${host}:${port}`);
    }
    return <Item variant="outline" >
        <ItemMedia variant="icon">
            {loading === true ? (
                <LoaderPinwheel className="animate-spin" />
            ) : (port ? (
                <CheckCircle2 className="text-green-500" />
            ) : (
                <XCircle className="text-red-500" />
            )
            )}
        </ItemMedia>
        <ItemContent>
            <ItemTitle>{port || "-"}</ItemTitle>
            <ItemDescription className="text-xs">
                {portType === "http" ? "HTTP/HTTPS" : ""}
                {portType === "socks" ? "SOCKS" : ""}
            </ItemDescription>
        </ItemContent>
        <ItemActions className={cn({ 'hidden': loading })}>
            <Button size="icon-sm" variant="ghost" onClick={copyHandler}>
                <Copy className="size-3" />
            </Button>
        </ItemActions>
    </Item>;
}