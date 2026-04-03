import { useEffect, useState } from 'react';
import { useAppStore } from '../store';
import { Clock, Sparkles, ChevronRight, ExternalLink, BookOpen, Code, Palette, Leaf, Gamepad2, Globe, Search, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent } from '@/components/ui/card';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogMedia, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { ThemeToggle } from '@/components/ThemeToggle';

const CATEGORY_ICONS: Record<string, React.ReactNode> = {
  '科普探索': <Globe className="h-5 w-5" />,
  '编程学习': <Code className="h-5 w-5" />,
  '艺术文化': <Palette className="h-5 w-5" />,
  '自然生态': <Leaf className="h-5 w-5" />,
  '益智游戏': <Gamepad2 className="h-5 w-5" />,
  '在线阅读': <BookOpen className="h-5 w-5" />,
};

const CATEGORY_COLORS: Record<string, string> = {
  '科普探索': 'from-blue-500/20 to-cyan-500/20 border-blue-500/30',
  '编程学习': 'from-purple-500/20 to-pink-500/20 border-purple-500/30',
  '艺术文化': 'from-orange-500/20 to-amber-500/20 border-orange-500/30',
  '自然生态': 'from-emerald-500/20 to-green-500/20 border-emerald-500/30',
  '益智游戏': 'from-rose-500/20 to-red-500/20 border-rose-500/30',
  '在线阅读': 'from-indigo-500/20 to-violet-500/20 border-indigo-500/30',
};

export function TeenDashboard() {
  const { filterState, recommendedSites, loadRecommendedSites, setMode } = useAppStore();
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [showAdminDialog, setShowAdminDialog] = useState(false);

  useEffect(() => {
    loadRecommendedSites();
  }, [loadRecommendedSites]);

  const handleAdminAccess = async () => {
    await setMode('parent');
  };

  const remainingHours = Math.floor(filterState.remaining_minutes / 60);
  const remainingMins = filterState.remaining_minutes % 60;

  const categories = Array.from(new Set(recommendedSites.map(s => s.category)));

  const filteredSites = selectedCategory
    ? recommendedSites.filter(s => s.category === selectedCategory)
    : recommendedSites;

  const searchedSites = searchQuery
    ? filteredSites.filter(s =>
        s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.description.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : filteredSites;

  const getCategorySites = (category: string) =>
    recommendedSites.filter(s => s.category === category).slice(0, 3);

  const openSite = (url: string) => {
    window.open(url, '_blank');
  };

  return (
    <div className="min-h-screen bg-background">
      <div className="max-w-5xl mx-auto px-4 py-8">
        <header className="flex items-center justify-between mb-8">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-emerald-500/20">
              <Sparkles className="h-6 w-6 text-emerald-400" />
            </div>
            <span className="text-foreground font-medium">成长伙伴</span>
          </div>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 bg-muted rounded-full px-4 py-2">
              <Clock className="h-4 w-4 text-emerald-400" />
              <span className="text-foreground font-mono">
                {remainingHours}:{remainingMins.toString().padStart(2, '0')}
              </span>
            </div>
            <ThemeToggle />
            <Button
              variant="outline"
              size="icon"
              onClick={() => setShowAdminDialog(true)}
              className="border-slate-700 text-slate-400 hover:text-foreground hover:bg-muted"
            >
              <Settings className="h-5 w-5" />
            </Button>
          </div>
        </header>

        <div className="text-center mb-10">
          <h1 className="text-3xl font-bold text-foreground mb-3">
            今天想探索什么？
          </h1>
          <p className="text-muted-foreground">
            发现精彩内容，让成长时光更有意义
          </p>
        </div>

        <div className="mb-8">
          <div className="relative max-w-md mx-auto">
            <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
            <Input
              type="text"
              placeholder="搜索你想了解的内容..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-12 pr-4 py-6 rounded-2xl"
            />
          </div>
        </div>

        {searchQuery ? (
          <div className="space-y-4">
            <h2 className="text-lg font-medium text-foreground flex items-center gap-2">
              <Search className="h-5 w-5 text-muted-foreground" />
              搜索结果
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {searchedSites.map((site) => (
                <Card key={site.id} className="cursor-pointer hover:bg-muted/50 transition-colors">
                  <CardContent className="p-5" onClick={() => openSite(site.url)}>
                    <div className="flex items-start gap-4">
                      <div className="text-3xl">{site.icon}</div>
                      <div className="flex-1 min-w-0">
                        <h3 className="font-medium text-foreground">
                          {site.title}
                        </h3>
                        <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                          {site.description}
                        </p>
                        <div className="flex items-center gap-1 mt-2 text-xs text-muted-foreground">
                          <span className="px-2 py-0.5 rounded-full bg-muted">
                            {site.category}
                          </span>
                          <ExternalLink className="h-3 w-3" />
                        </div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
            {searchedSites.length === 0 && (
              <Card className="bg-card">
                <CardContent className="py-12 text-center text-muted-foreground">
                  <Search className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>没有找到相关推荐</p>
                </CardContent>
              </Card>
            )}
          </div>
        ) : selectedCategory ? (
          <div className="space-y-4">
            <Button
              variant="ghost"
              onClick={() => setSelectedCategory(null)}
              className="text-muted-foreground hover:text-foreground"
            >
              ← 返回分类
            </Button>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {searchedSites.map((site) => (
                <Card key={site.id} className="cursor-pointer hover:bg-muted/50 transition-colors">
                  <CardContent className="p-5" onClick={() => openSite(site.url)}>
                    <div className="flex items-start gap-4">
                      <div className="text-3xl">{site.icon}</div>
                      <div className="flex-1 min-w-0">
                        <h3 className="font-medium text-foreground">
                          {site.title}
                        </h3>
                        <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                          {site.description}
                        </p>
                        <div className="flex items-center gap-1 mt-3 text-xs text-emerald-400">
                          <span>去看看</span>
                          <ChevronRight className="h-3 w-3" />
                        </div>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        ) : (
          <div className="space-y-8">
            {categories.map((category) => (
              <div key={category}>
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-medium text-foreground flex items-center gap-2">
                    <span className={`p-2 rounded-lg bg-gradient-to-br ${CATEGORY_COLORS[category] || 'bg-muted'}`}>
                      {CATEGORY_ICONS[category] || <Globe className="h-5 w-5" />}
                    </span>
                    {category}
                  </h2>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setSelectedCategory(category)}
                    className="text-muted-foreground hover:text-emerald-400"
                  >
                    查看全部
                    <ChevronRight className="h-4 w-4 ml-1" />
                  </Button>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                  {getCategorySites(category).map((site) => (
                    <Card key={site.id} className="cursor-pointer hover:bg-muted/50 transition-colors">
                      <CardContent className="p-5" onClick={() => openSite(site.url)}>
                        <div className="flex items-start gap-4">
                          <div className="text-3xl">{site.icon}</div>
                          <div className="flex-1 min-w-0">
                            <h3 className="font-medium text-foreground">
                              {site.title}
                            </h3>
                            <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                              {site.description}
                            </p>
                            <div className="flex items-center gap-1 mt-3 text-xs text-emerald-400">
                              <span>去看看</span>
                              <ChevronRight className="h-3 w-3" />
                            </div>
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        <footer className="mt-16 text-center text-sm text-muted-foreground">
          探索世界 · 快乐成长
        </footer>
      </div>

      <AlertDialog open={showAdminDialog} onOpenChange={setShowAdminDialog}>
        <AlertDialogContent className="sm:max-w-[400px]">
          <AlertDialogHeader>
            <AlertDialogMedia className="bg-amber-500/20">
              <Settings className="h-8 w-8 text-amber-400" />
            </AlertDialogMedia>
            <AlertDialogTitle>家长入口</AlertDialogTitle>
            <AlertDialogDescription>
              进入家长模式进行管理设置
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction onClick={handleAdminAccess}>
              进入管理
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
