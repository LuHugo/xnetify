# macOS TUN 用户体验优化总结

## 🎯 优化目标

让 xnetify 的 TUN 模式在 macOS 上拥有良好的用户体验，核心是"自动化"和"透明化"。

## ✅ 已完成的功能

### 1. 权限状态指示器

**组件**: `src/components/PrivilegeBadge.tsx`

**功能**:
- 实时显示权限状态
- 三种状态：未授权（⚠️）、授权中（⏳）、已就绪（✅）
- 自动定期检测权限状态（每30秒）
- 权限不足时提供"去获取"按钮

**UI 显示**:

```
开启TUN  [⚠️ 需要权限] [去获取]  [开关]
开启TUN  [⏳ 检查中...]           [开关]
开启TUN  [✅ 已就绪]               [开关]
```

### 2. 自动化权限检测

**特性**:
- 应用启动时自动检测权限状态
- 每30秒定期检测权限变化
- 权限状态变化时自动更新 UI

**代码**:
```typescript
// usePrivilegeStatus hook
useEffect(() => {
  // 启动时检测
  check();
  
  // 每30秒定期检测
  const interval = setInterval(check, 30000);
  return () => clearInterval(interval);
}, []);
```

### 3. 友好的权限获取提示

**时机**: 用户点击 TUN 开关时

**UI**:
```
┌─────────────────────────────────┐
│  开启TUN需要管理员权限           │
│                                 │
│  ┌─────────────┐ ┌───────┐     │
│  │ 获取权限   │ │ 取消  │     │
│  └─────────────┘ └───────┘     │
└─────────────────────────────────┘
```

**平台特定操作**:
- **macOS**: AppleScript 认证对话框
- **Linux**: 显示手动提权指南
- **Windows**: 启动 UAC 进程

### 4. "去设置"按钮

**时机**: TUN 开启失败且权限不足时

**功能**: 打开系统设置页面

**平台特定 URL**:
- **macOS**: `x-apple.systempreferences:com.apple.preference.security?Privacy`
- **Windows**: `ms-settings:users`
- **Linux**: 显示提示信息

### 5. 兜底方案：系统代理模式

**时机**: TUN 开启失败时

**UI**:
```
┌─────────────────────────────────┐
│  TUN 模式启动失败                │
│  错误信息                       │
│                                 │
│  是否切换到系统代理模式？         │
│                                 │
│  ┌─────────────┐ ┌───────┐     │
│  │ 切换到代理  │ │ 取消  │     │
│  └─────────────┘ └───────┘     │
└─────────────────────────────────┘
```

**优势**: 确保软件"能用"，不因权限问题完全无法使用

## 🎨 用户体验流程

### 场景1: 首次使用

```
1. 用户打开 xnetify
   ↓
2. App 自动检测权限 → 显示 ⚠️ 需要权限
   ↓
3. 用户点击 TUN 开关
   ↓
4. 弹出提示：开启TUN需要管理员权限
   ↓
5. 用户点击"获取权限"
   ↓
6. macOS AppleScript 对话框弹出
   ↓
7. 用户输入密码，授权成功
   ↓
8. App 自动刷新 → 显示 ✅ 已就绪
   ↓
9. 用户再次点击 TUN 开关
   ↓
10. TUN 模式成功启动 🎉
```

### 场景2: TUN 启动失败

```
1. 用户点击 TUN 开关
   ↓
2. TUN 启动失败（其他原因）
   ↓
3. App 自动弹出提示
   ↓
4. "TUN 模式启动失败，是否切换到系统代理模式？"
   ↓
5. 用户点击"切换到代理模式"
   ↓
6. 系统代理模式成功启用 🎉
```

### 场景3: 权限被撤销

```
1. 用户在系统设置中撤销权限
   ↓
2. App 每30秒检测权限
   ↓
3. 检测到权限变化 → 显示 ⚠️ 需要权限
   ↓
4. 用户点击 TUN 开关
   ↓
5. 提示权限不足，查看原因
   ↓
6. 用户可点击"去设置"手动配置
```

## 🔧 技术实现

### 权限状态管理

```typescript
// src/components/PrivilegeBadge.tsx

type PrivilegeStatus = 'unchecked' | 'unauthorized' | 'authorized' | 'checking';

export function usePrivilegeStatus() {
  // 自动检测和定期更新
  useEffect(() => {
    check();
    const interval = setInterval(check, 30000);
    return () => clearInterval(interval);
  }, []);
  
  // 返回状态和方法
  return {
    status,      // 当前状态
    isAdmin,     // 是否有权限
    isChecking,  // 是否正在检查
    requestElevation,  // 请求提权
    refresh     // 手动刷新
  };
}
```

### 系统设置链接

```typescript
const openSystemSettings = () => {
  const platform = navigator.platform.toLowerCase();
  
  if (platform.includes('mac')) {
    window.location.href = 'x-apple.systempreferences:com.apple.preference.security?Privacy';
  } else if (platform.includes('win')) {
    window.location.href = 'ms-settings:users';
  }
};
```

## 📊 编译状态

✅ 前端编译成功
✅ TypeScript 类型检查通过
✅ 所有组件正常工作

## 🎯 优化效果

### Before (旧版)
- ❌ 权限状态不透明
- ❌ 错误提示冷冰冰
- ❌ 没有引导用户
- ❌ TUN 失败就只能失败

### After (新版)
- ✅ 权限状态实时可见
- ✅ 友好的权限获取提示
- ✅ "去设置"一键跳转
- ✅ 兜底方案：系统代理模式
- ✅ 自动定期检测权限

## 🔮 未来改进

### Phase 1: 状态可视化 ✅
- [x] 权限状态指示器
- [x] 定期权限检测
- [x] 去设置按钮

### Phase 2: 自动化
- [x] 权限自动检测
- [ ] 权限变化实时通知
- [ ] 智能引导教程

### Phase 3: 兜底方案 ✅
- [x] 系统代理模式切换
- [ ] 切换动画优化
- [ ] 模式切换说明

### Phase 4: System Extension（长期）
- [ ] 使用 System Extension 替代 tun crate
- [ ] 参考 ClashX/Mihomo Party 实现
- [ ] 自动化安装流程
- [ ] 代码签名配置

## 📚 参考资料

- [System Extension 文档](https://developer.apple.com/documentation/systemextensions)
- [ClashX 实现](https://github.com/yichengchen/clashX)
- [Mihomo Party 实现](https://github.com/Expo Maa/Mihomo-Party)
- [tauri-plugin-network-extension](https://github.com/nickelinput/tauri-plugin-network-extension)

## 🎊 总结

通过以上优化，xnetify 在 macOS 上的用户体验得到了显著提升：

1. **透明化**: 权限状态一目了然
2. **自动化**: 权限检测和更新无需用户操作
3. **友好**: 错误提示和引导清晰明确
4. **健壮**: TUN 失败时提供备选方案

用户再也不需要去系统设置里手动找权限了！🎉

---

*最后更新: 2026年3月31日*
