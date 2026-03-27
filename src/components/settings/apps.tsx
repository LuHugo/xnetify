import { cn } from "@/lib/utils";

export default function Apps({ show=false }: { show?: boolean }) {
    return (
        <div className={cn({ "hidden": !show })}>Apps</div>
    );
}
