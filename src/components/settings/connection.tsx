import { cn } from "@/lib/utils";

export default function Connection({ show=false }: { show?: boolean }) {
    return (
        <div className={cn({ "hidden": !show })}>Connection</div>
    );
}