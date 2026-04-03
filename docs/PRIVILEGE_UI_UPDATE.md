# 权限提示 UI 更新

## 变更日期
2026年3月31日

## 变更内容

### 简化权限提示

**之前**: 使用 toast 的 action 按钮
```typescript
toast.info('需要管理员权限', {
  description: '正在请求权限提升...',
  action: {
    label: '获取权限',
    onClick: async () => { ... }
  },
  duration: 10000
});
```

**现在**: 使用自定义 Toast，包含明确的文本和按钮
```typescript
toast.custom((t) => (
  <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg border p-4 max-w-sm">
    <p className="text-sm font-medium mb-3">
      开启TUN需要管理员权限
    </p>
    <div className="flex gap-2">
      <button className="flex-1 bg-blue-500 text-white px-4 py-2 rounded-md">
        获取权限
      </button>
      <button className="bg-gray-100 dark:bg-gray-700 px-4 py-2 rounded-md">
        取消
      </button>
    </div>
  </div>
), { duration: 10000 });
```

## 用户体验

### 权限不足提示

当用户点击"TUN 开启"且没有管理员权限时：

```
┌─────────────────────────────────┐
│                                 │
│  开启TUN需要管理员权限           │
│                                 │
│  ┌─────────────┐ ┌───────┐    │
│  │  获取权限   │ │ 取消  │    │
│  └─────────────┘ └───────┘    │
│                                 │
└─────────────────────────────────┘
```

### 点击"获取权限"后

**macOS**: 
- 弹出 AppleScript 认证对话框
- 用户输入密码
- 成功 → toast 提示"请在新的管理员窗口中使用 TUN 功能"

**Linux**:
- 显示错误信息（Linux 不支持自动提权）

**Windows**:
- 启动 UAC 对话框
- 用户确认
- 成功 → toast 提示"请在新的管理员窗口中使用 TUN 功能"

### 点击"取消"后

- Toast 关闭
- TUN 开关保持关闭状态

## 技术细节

### 使用 sonner 的 toast.custom

```typescript
const toastId = toast.custom((t) => (
  // 自定义 JSX 内容
  // t 是 toast ID，可用于手动关闭
), { duration: 10000 });

// 关闭 toast
toast.dismiss(toastId);
```

### 样式

- 使用 Tailwind CSS 样式
- 支持深色模式 (`dark:` 前缀)
- 圆角边框
- 阴影效果
- 响应式布局

## 文件变更

- `src/App.tsx` - 修改 `handleTunSwitch` 函数

## 编译状态

✅ 前端编译成功

## 下一步

1. 测试不同平台的权限提交流程
2. 优化错误提示信息
3. 添加权限状态指示器
