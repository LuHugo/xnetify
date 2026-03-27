import { cn } from "@/lib/utils";

export default function Proxies({ show=false }: { show?: boolean }) {
    return (
        <div className={cn({ "hidden": !show })}>Proxies</div>
    );
}