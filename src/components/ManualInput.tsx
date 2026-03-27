import { FC, useState } from "react";

interface ManualInputProps {
  onSubmit: (httpPort: number, socksPort: number) => void;
}

const ManualInput: FC<ManualInputProps> = ({ onSubmit }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [httpPort, setHttpPort] = useState("7890");
  const [socksPort, setSocksPort] = useState("7892");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const http = parseInt(httpPort) || 7890;
    const socks = parseInt(socksPort) || 7892;
    onSubmit(http, socks);
    setIsOpen(false);
  };

  return (
    <div className="border border-gray-200 rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-4 py-2.5 flex items-center justify-between text-sm text-gray-600 hover:bg-gray-50 transition-colors"
      >
        <span>手动输入端口</span>
        <svg
          className={`w-4 h-4 transition-transform ${isOpen ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {isOpen && (
        <form onSubmit={handleSubmit} className="p-4 bg-gray-50 border-t border-gray-200">
          <div className="grid grid-cols-2 gap-3 mb-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">HTTP 端口</label>
              <input
                type="number"
                value={httpPort}
                onChange={(e) => setHttpPort(e.target.value)}
                className="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                placeholder="7890"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">SOCKS 端口</label>
              <input
                type="number"
                value={socksPort}
                onChange={(e) => setSocksPort(e.target.value)}
                className="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                placeholder="7892"
              />
            </div>
          </div>
          <button
            type="submit"
            className="w-full px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-sm font-medium rounded-lg transition-colors"
          >
            应用
          </button>
        </form>
      )}
    </div>
  );
};

export default ManualInput;
