import { FC } from "react";

interface ProxyApp {
  name: string;
  http_port: number | null;
  socks_port: number | null;
  pid: number | null;
}

interface ActionButtonsProps {
  apps: ProxyApp[];
  onCopy: (type: "set" | "unset") => void;
  onNewTerminal: (type: "set" | "unset") => void;
  isWindows: boolean;
}

const ActionButtons: FC<ActionButtonsProps> = ({ apps, onCopy, onNewTerminal, isWindows }) => {
  const hasApps = apps.length > 0;
  const shellType = isWindows ? "powershell" : "bash";

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <button
          onClick={() => onCopy("set")}
          disabled={!hasApps}
          className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg font-medium transition-all ${
            hasApps
              ? "bg-blue-500 hover:bg-blue-600 text-white shadow-sm hover:shadow"
              : "bg-gray-100 text-gray-400 cursor-not-allowed"
          }`}
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
          </svg>
          复制命令
        </button>
        <button
          onClick={() => onCopy("unset")}
          disabled={!hasApps}
          className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg font-medium transition-all ${
            hasApps
              ? "bg-gray-100 hover:bg-gray-200 text-gray-700 border border-gray-200"
              : "bg-gray-50 text-gray-400 cursor-not-allowed"
          }`}
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
          撤销
        </button>
      </div>

      {isWindows && (
        <button
          onClick={() => onNewTerminal("set")}
          disabled={!hasApps}
          className={`w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg font-medium transition-all ${
            hasApps
              ? "bg-green-500 hover:bg-green-600 text-white shadow-sm hover:shadow"
              : "bg-gray-100 text-gray-400 cursor-not-allowed"
          }`}
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
          新开终端
        </button>
      )}

      <p className="text-xs text-gray-400 text-center">
        Shell 类型: {shellType}
      </p>
    </div>
  );
};

export default ActionButtons;
