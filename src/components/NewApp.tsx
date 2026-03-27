import { useState } from "react";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "./ui/dialog";
import { Field, FieldGroup } from "./ui/field";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./ui/accordion";
import { Button } from "./ui/button";
import { PlusCircle } from "lucide-react";
import { useAppStore, type ProxyApp } from "@/store";

export default function NewApp() {
    const { setManualApps } = useAppStore();
    const [open, setOpen] = useState(false);
    const [name, setName] = useState("");
    const [httpHost, setHttpHost] = useState("127.0.0.1");
    const [httpPort, setHttpPort] = useState("");
    const [socksHost, setSocksHost] = useState("127.0.0.1");
    const [socksPort, setSocksPort] = useState("");

    const handleSave = () => {
        if (!name.trim()) return;

        const allPorts: number[] = [];
        if (httpPort) allPorts.push(parseInt(httpPort));
        if (socksPort) allPorts.push(parseInt(socksPort));

        const newApp: ProxyApp = {
            name: name.trim(),
            host: httpHost.trim() || "127.0.0.1",
            http_port: httpPort ? parseInt(httpPort) : null,
            socks_port: socksPort ? parseInt(socksPort) : null,
            all_ports: allPorts,
            pid: null,
            tested_http: null,
            tested_socks: null,
            tested: null,
            latency_ms: null,
        };

        setManualApps([newApp]);
        setOpen(false);
        setName("");
        setHttpHost("127.0.0.1");
        setHttpPort("");
        setSocksHost("127.0.0.1");
        setSocksPort("");
    };

    const handleCancel = () => {
        setOpen(false);
        setName("");
        setHttpHost("127.0.0.1");
        setHttpPort("");
        setSocksHost("127.0.0.1");
        setSocksPort("");
    };

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger render={<Button variant="ghost" size="icon" title="添加">
                <PlusCircle className="h-5 w-5" />
            </Button>} />
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>添加代理应用</DialogTitle>
                </DialogHeader>
                <FieldGroup>
                    <Field>
                        <Label htmlFor="name">名称</Label>
                        <Input
                            id="name"
                            name="name"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            placeholder="Clash Verge"
                        />
                    </Field>
                    <Accordion defaultValue={["http"]} className="max-w-lg">
                        <AccordionItem value="http">
                            <AccordionTrigger>HTTP/HTTPS 代理</AccordionTrigger>
                            <AccordionContent className="px-1 space-y-2">
                                <Field>
                                    <Label htmlFor="httpHost">主机</Label>
                                    <Input
                                        id="httpHost"
                                        name="httpHost"
                                        value={httpHost}
                                        onChange={(e) => setHttpHost(e.target.value)}
                                        placeholder="127.0.0.1"
                                    />
                                </Field>
                                <Field>
                                    <Label htmlFor="httpPort">端口</Label>
                                    <Input
                                        id="httpPort"
                                        name="httpPort"
                                        type="number"
                                        value={httpPort}
                                        onChange={(e) => setHttpPort(e.target.value)}
                                        placeholder="7890"
                                    />
                                </Field>
                            </AccordionContent>
                        </AccordionItem>
                        <AccordionItem value="socks">
                            <AccordionTrigger>SOCKS 代理</AccordionTrigger>
                            <AccordionContent className="px-1 space-y-2">
                                <Field>
                                    <Label htmlFor="socksHost">主机</Label>
                                    <Input
                                        id="socksHost"
                                        name="socksHost"
                                        value={socksHost}
                                        onChange={(e) => setSocksHost(e.target.value)}
                                        placeholder="127.0.0.1"
                                    />
                                </Field>
                                <Field>
                                    <Label htmlFor="socksPort">端口</Label>
                                    <Input
                                        id="socksPort"
                                        name="socksPort"
                                        type="number"
                                        value={socksPort}
                                        onChange={(e) => setSocksPort(e.target.value)}
                                        placeholder="7892"
                                    />
                                </Field>
                            </AccordionContent>
                        </AccordionItem>
                    </Accordion>
                </FieldGroup>
                <DialogFooter>
                    <Button variant="ghost" onClick={handleCancel}>
                        取消
                    </Button>
                    <Button onClick={handleSave} disabled={!name.trim()}>
                        保存
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
