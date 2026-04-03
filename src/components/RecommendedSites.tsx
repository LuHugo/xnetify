import { useState } from 'react';
import { useAppStore, RecommendedSite } from '../store';
import { Card, CardContent, CardHeader, CardTitle } from './ui/card';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Globe, Plus, Trash2, Edit2, Save, X, Eye, EyeOff, ExternalLink } from 'lucide-react';

const CATEGORIES = [
  '科普探索',
  '编程学习',
  '艺术文化',
  '自然生态',
  '益智游戏',
  '在线阅读',
];

export function RecommendedSites() {
  const { recommendedSites, addRecommendedSite, removeRecommendedSite, updateRecommendedSite, mode } = useAppStore();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [editingSite, setEditingSite] = useState<RecommendedSite | null>(null);
  const [newSite, setNewSite] = useState<Partial<RecommendedSite>>({
    title: '',
    description: '',
    url: '',
    icon: '🌟',
    category: '科普探索',
    enabled: true,
  });

  const handleAddSite = async () => {
    if (!newSite.title || !newSite.url) return;

    const site: RecommendedSite = {
      id: crypto.randomUUID(),
      title: newSite.title,
      description: newSite.description || '',
      url: newSite.url,
      icon: newSite.icon || '🌟',
      category: newSite.category || '科普探索',
      enabled: true,
    };

    await addRecommendedSite(site);
    setShowAddDialog(false);
    setNewSite({
      title: '',
      description: '',
      url: '',
      icon: '🌟',
      category: '科普探索',
      enabled: true,
    });
  };

  const handleUpdateSite = async () => {
    if (!editingSite) return;
    await updateRecommendedSite(editingSite);
    setEditingSite(null);
  };

  const handleToggleEnabled = async (site: RecommendedSite) => {
    await updateRecommendedSite({ ...site, enabled: !site.enabled });
  };

  const isReadOnly = mode === 'teen';

  const groupedSites = recommendedSites.reduce((acc, site) => {
    if (!acc[site.category]) {
      acc[site.category] = [];
    }
    acc[site.category].push(site);
    return acc;
  }, {} as Record<string, RecommendedSite[]>);

  return (
    <Card className="bg-slate-900/50 border-slate-800 backdrop-blur-sm">
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Globe className="h-5 w-5 text-emerald-400" />
            精彩内容推荐
          </div>
          {!isReadOnly && (
            <Button
              size="sm"
              onClick={() => setShowAddDialog(true)}
              className="bg-emerald-500 hover:bg-emerald-600 text-white"
            >
              <Plus className="h-4 w-4 mr-1" />
              添加内容
            </Button>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {Object.entries(groupedSites).map(([category, sites]) => (
            <div key={category}>
              <h3 className="text-sm font-medium text-slate-400 mb-3">{category}</h3>
              <div className="space-y-2">
                {sites.map((site) => (
                  <div
                    key={site.id}
                    className={`p-3 rounded-xl flex items-center justify-between ${
                      site.enabled ? 'bg-slate-800/50' : 'bg-slate-800/20 opacity-50'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span className="text-2xl">{site.icon}</span>
                      <div>
                        <p className="font-medium text-white">{site.title}</p>
                        <p className="text-xs text-slate-400 truncate max-w-[200px]">
                          {site.url}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <a
                        href={site.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="p-2 text-slate-400 hover:text-emerald-400 transition-colors"
                      >
                        <ExternalLink className="h-4 w-4" />
                      </a>
                      {!isReadOnly && (
                        <>
                          <button
                            onClick={() => handleToggleEnabled(site)}
                            className={`p-2 rounded-lg transition-colors ${
                              site.enabled
                                ? 'text-emerald-400 hover:bg-emerald-500/20'
                                : 'text-slate-500 hover:bg-slate-700'
                            }`}
                          >
                            {site.enabled ? (
                              <Eye className="h-4 w-4" />
                            ) : (
                              <EyeOff className="h-4 w-4" />
                            )}
                          </button>
                          <button
                            onClick={() => removeRecommendedSite(site.id)}
                            className="p-2 text-slate-400 hover:text-rose-400 hover:bg-rose-500/20 rounded-lg transition-colors"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}

          {recommendedSites.length === 0 && (
            <div className="text-center py-8 text-slate-500">
              暂无推荐内容
            </div>
          )}
        </div>
      </CardContent>

      {showAddDialog && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50">
          <Card className="w-[420px] bg-slate-900 border-slate-800">
            <CardHeader className="pb-4">
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Plus className="h-5 w-5 text-emerald-400" />
                  添加推荐内容
                </CardTitle>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowAddDialog(false)}
                  className="text-slate-400 hover:text-white"
                >
                  <X className="h-5 w-5" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <label className="text-sm text-slate-400">名称</label>
                <Input
                  placeholder="例如: 国家地理"
                  value={newSite.title}
                  onChange={(e) => setNewSite({ ...newSite, title: e.target.value })}
                  className="bg-slate-800 border-slate-700 text-white"
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm text-slate-400">网址</label>
                <Input
                  placeholder="https://example.com"
                  value={newSite.url}
                  onChange={(e) => setNewSite({ ...newSite, url: e.target.value })}
                  className="bg-slate-800 border-slate-700 text-white"
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm text-slate-400">描述（可选）</label>
                <Input
                  placeholder="内容简介"
                  value={newSite.description}
                  onChange={(e) => setNewSite({ ...newSite, description: e.target.value })}
                  className="bg-slate-800 border-slate-700 text-white"
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm text-slate-400">分类</label>
                <div className="flex flex-wrap gap-2">
                  {CATEGORIES.map((cat) => (
                    <button
                      key={cat}
                      type="button"
                      onClick={() => setNewSite({ ...newSite, category: cat })}
                      className={`px-3 py-1.5 text-sm rounded-lg transition-all ${
                        newSite.category === cat
                          ? 'bg-emerald-500 text-white'
                          : 'bg-slate-800 text-slate-400 hover:bg-slate-700'
                      }`}
                    >
                      {cat}
                    </button>
                  ))}
                </div>
              </div>
              <div className="space-y-2">
                <label className="text-sm text-slate-400">图标（emoji）</label>
                <Input
                  placeholder="🌟"
                  value={newSite.icon}
                  onChange={(e) => setNewSite({ ...newSite, icon: e.target.value })}
                  className="bg-slate-800 border-slate-700 text-white w-20 text-center text-2xl"
                  maxLength={2}
                />
              </div>
              <div className="flex gap-3 pt-4">
                <Button
                  variant="secondary"
                  className="flex-1 bg-slate-800 hover:bg-slate-700 text-white"
                  onClick={() => setShowAddDialog(false)}
                >
                  取消
                </Button>
                <Button
                  className="flex-1 bg-emerald-500 hover:bg-emerald-600 text-white"
                  onClick={handleAddSite}
                  disabled={!newSite.title || !newSite.url}
                >
                  添加
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </Card>
  );
}
