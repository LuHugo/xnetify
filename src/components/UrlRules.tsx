import { useState } from 'react';
import { useAppStore, UrlRule, UrlRuleType } from '../store';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Globe, Plus, Trash2, Ban, CheckCircle, Search, X } from 'lucide-react';

const CATEGORIES = [
  { value: 'porn', label: '色情', color: 'bg-rose-500' },
  { value: 'gambling', label: '赌博', color: 'bg-orange-500' },
  { value: 'violence', label: '暴力', color: 'bg-yellow-500' },
  { value: 'drugs', label: '毒品', color: 'bg-purple-500' },
  { value: 'hate', label: '仇恨', color: 'bg-pink-500' },
  { value: 'social', label: '社交', color: 'bg-blue-500' },
  { value: 'gaming', label: '游戏', color: 'bg-green-500' },
  { value: 'video', label: '视频', color: 'bg-cyan-500' },
];

export function UrlRules() {
  const { urlRules, addUrlRule, removeUrlRule, mode, checkUrl } = useAppStore();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [newRule, setNewRule] = useState<Partial<UrlRule>>({
    name: '',
    pattern: '',
    rule_type: 'block',
    category: null,
    enabled: true,
  });
  const [testUrl, setTestUrl] = useState('');
  const [testResult, setTestResult] = useState<{ allowed: boolean; reason?: string } | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isAdding, setIsAdding] = useState(false);

  const handleAddRule = async () => {
    if (!newRule.name || !newRule.pattern) return;

    setIsAdding(true);
    try {
      const rule: UrlRule = {
        id: crypto.randomUUID(),
        name: newRule.name,
        pattern: newRule.pattern,
        rule_type: newRule.rule_type || 'block',
        category: newRule.category || null,
        enabled: true,
      };

      await addUrlRule(rule);
      setShowAddDialog(false);
      setNewRule({
        name: '',
        pattern: '',
        rule_type: 'block',
        category: null,
        enabled: true,
      });
    } finally {
      setIsAdding(false);
    }
  };

  const handleTestUrl = async () => {
    if (!testUrl) return;
    setIsTesting(true);
    try {
      const result = await checkUrl(testUrl);
      setTestResult(result);
    } finally {
      setIsTesting(false);
    }
  };

  const isReadOnly = mode === 'teen';

  return (
    <Card className="bg-slate-900/50 border-slate-800">
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2">
          <Globe className="h-5 w-5 text-emerald-400" />
          内容守护设置
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-500" />
            <Input
              placeholder="输入网址测试..."
              value={testUrl}
              onChange={(e) => {
                setTestUrl(e.target.value);
                setTestResult(null);
              }}
              onKeyDown={(e) => e.key === 'Enter' && handleTestUrl()}
              className="pl-10"
            />
          </div>
          <Button 
            onClick={handleTestUrl} 
            variant="secondary"
            disabled={isTesting || !testUrl}
          >
            {isTesting ? '测试中...' : '测试'}
          </Button>
        </div>

        {testResult && (
          <div
            className={`p-4 rounded-xl ${
              testResult.allowed
                ? 'bg-emerald-500/20 border border-emerald-500/30'
                : 'bg-rose-500/20 border border-rose-500/30'
            }`}
          >
            <div className="flex items-center gap-3">
              {testResult.allowed ? (
                <CheckCircle className="h-6 w-6 text-emerald-400" />
              ) : (
                <Ban className="h-6 w-6 text-rose-400" />
              )}
              <div>
                <p className={`font-medium ${
                  testResult.allowed ? 'text-emerald-400' : 'text-rose-400'
                }`}>
                  {testResult.allowed ? '可以访问' : '暂不推荐'}
                </p>
                {testResult.reason && (
                  <p className="text-sm text-muted-foreground mt-1">{testResult.reason}</p>
                )}
              </div>
            </div>
          </div>
        )}

        <div className="space-y-2">
          {urlRules.length === 0 ? (
            <div className="p-6 text-center text-muted-foreground">
              暂无自定义规则
            </div>
          ) : (
            urlRules.map((rule) => (
              <div
                key={rule.id}
                className={`p-4 rounded-xl bg-muted/50 flex items-center justify-between ${
                  !rule.enabled ? 'opacity-50' : ''
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`p-2 rounded-lg ${
                      rule.rule_type === 'block'
                        ? 'bg-rose-500/20'
                        : 'bg-emerald-500/20'
                    }`}
                  >
                    {rule.rule_type === 'block' ? (
                      <Ban className={`h-4 w-4 ${
                        rule.rule_type === 'block'
                          ? 'text-rose-400'
                          : 'text-emerald-400'
                      }`} />
                    ) : (
                      <CheckCircle className="h-4 w-4 text-emerald-400" />
                    )}
                  </div>
                  <div>
                    <p className="font-medium">{rule.name}</p>
                    <p className="text-sm text-muted-foreground">{rule.pattern}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {rule.category && (
                    <span
                      className={`px-2 py-1 text-xs rounded text-white ${
                        CATEGORIES.find((c) => c.value === rule.category)?.color ||
                        'bg-gray-500'
                      }`}
                    >
                      {CATEGORIES.find((c) => c.value === rule.category)?.label}
                    </span>
                  )}
                  {!isReadOnly && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => removeUrlRule(rule.id)}
                      className="text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              </div>
            ))
          )}
        </div>

        {!isReadOnly && (
          <Button
            variant="outline"
            onClick={() => setShowAddDialog(true)}
            className="w-full border-dashed"
          >
            <Plus className="h-4 w-4 mr-2" />
            添加规则
          </Button>
        )}
      </CardContent>

      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent className="sm:max-w-[480px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Plus className="h-5 w-5 text-emerald-400" />
              添加内容规则
            </DialogTitle>
            <DialogDescription>
              设置网址匹配规则来限制或允许访问特定内容
            </DialogDescription>
          </DialogHeader>
          
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="rule-name">规则名称</Label>
              <Input
                id="rule-name"
                placeholder="例如: 不适合的内容"
                value={newRule.name}
                onChange={(e) =>
                  setNewRule({ ...newRule, name: e.target.value })
                }
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="rule-pattern">匹配模式</Label>
              <Input
                id="rule-pattern"
                placeholder="例如: example.com 或 *.example.com"
                value={newRule.pattern}
                onChange={(e) =>
                  setNewRule({ ...newRule, pattern: e.target.value })
                }
              />
              <p className="text-xs text-muted-foreground">
                支持通配符: * 匹配任意字符
              </p>
            </div>
            
            <div className="space-y-2">
              <Label>规则类型</Label>
              <div className="flex gap-4">
                <Button
                  type="button"
                  variant={newRule.rule_type === 'block' ? 'default' : 'outline'}
                  className="flex-1 gap-2"
                  onClick={() => setNewRule({ ...newRule, rule_type: 'block' })}
                >
                  <Ban className="h-4 w-4" />
                  限制
                </Button>
                <Button
                  type="button"
                  variant={newRule.rule_type === 'allow' ? 'default' : 'outline'}
                  className="flex-1 gap-2"
                  onClick={() => setNewRule({ ...newRule, rule_type: 'allow' })}
                >
                  <CheckCircle className="h-4 w-4" />
                  允许
                </Button>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="rule-category">分类标签（可选）</Label>
              <Select
                value={newRule.category || ''}
                onValueChange={(value) =>
                  setNewRule({ ...newRule, category: value || null })
                }
              >
                <SelectTrigger id="rule-category">
                  <SelectValue placeholder="选择分类" />
                </SelectTrigger>
                <SelectContent>
                  {CATEGORIES.map((cat) => (
                    <SelectItem key={cat.value} value={cat.value}>
                      {cat.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              variant="outline"
              onClick={() => setShowAddDialog(false)}
              className="flex-1"
            >
              取消
            </Button>
            <Button
              onClick={handleAddRule}
              disabled={!newRule.name || !newRule.pattern || isAdding}
              className="flex-1"
            >
              {isAdding ? '添加中...' : '添加'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
