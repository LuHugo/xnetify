import { useState } from 'react';
import { useAppStore, Mode } from '../store';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog';
import { Shield, Sun, Moon, Lock } from 'lucide-react';

export function ModeSwitch() {
  const { mode, setMode, verifyPin, filterState } = useAppStore();
  const [pinInput, setPinInput] = useState('');
  const [showPinDialog, setShowPinDialog] = useState(false);
  const [pinError, setPinError] = useState(false);
  const [isVerifying, setIsVerifying] = useState(false);

  const handleModeSwitch = async (newMode: Mode) => {
    if (newMode === 'parent') {
      setShowPinDialog(true);
      setPinError(false);
      setPinInput('');
    } else {
      await setMode(newMode);
    }
  };

  const handlePinSubmit = async () => {
    if (pinInput.length < 4) return;
    
    setIsVerifying(true);
    try {
      const valid = await verifyPin(pinInput);
      if (valid) {
        await setMode('parent');
        setShowPinDialog(false);
        setPinInput('');
      } else {
        setPinError(true);
      }
    } finally {
      setIsVerifying(false);
    }
  };

  const isParent = mode === 'parent';

  return (
    <>
      <Card className="bg-slate-900/50 border-slate-800">
        <CardHeader className="pb-4">
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5 text-blue-400" />
            守护模式
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className={`p-4 rounded-2xl ${
                isParent 
                  ? 'bg-gradient-to-br from-blue-500 to-indigo-600' 
                  : 'bg-gradient-to-br from-amber-500 to-orange-600'
              }`}>
                {isParent ? (
                  <Moon className="h-8 w-8 text-white" />
                ) : (
                  <Sun className="h-8 w-8 text-white" />
                )}
              </div>
              <div>
                <p className="text-xl font-bold">
                  {isParent ? '陪伴模式' : '成长模式'}
                </p>
                <p className="text-sm text-muted-foreground">
                  {isParent 
                    ? '家长陪伴中，可访问全部内容' 
                    : `今日成长时光 ${filterState.remaining_minutes} 分钟`
                  }
                </p>
              </div>
            </div>
            <Button
              variant={isParent ? "secondary" : "default"}
              onClick={() => handleModeSwitch(isParent ? 'teen' : 'parent')}
              className={isParent ? 'bg-slate-800 hover:bg-slate-700' : 'bg-amber-500 hover:bg-amber-600'}
            >
              {isParent ? (
                <>
                  <Sun className="h-4 w-4 mr-2" />
                  切换到成长模式
                </>
              ) : (
                <>
                  <Lock className="h-4 w-4 mr-2" />
                  解锁陪伴模式
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Dialog open={showPinDialog} onOpenChange={setShowPinDialog}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <div className="mx-auto p-3 rounded-full bg-blue-500/20 w-fit mb-4">
              <Lock className="h-8 w-8 text-blue-400" />
            </div>
            <DialogTitle className="text-center">输入家长密码</DialogTitle>
            <DialogDescription className="text-center">
              请输入家长密码进入陪伴模式
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              type="password"
              placeholder="请输入 PIN"
              maxLength={6}
              value={pinInput}
              onChange={(e) => {
                setPinInput(e.target.value.replace(/\D/g, ''));
                setPinError(false);
              }}
              onKeyDown={(e) => e.key === 'Enter' && handlePinSubmit()}
              className={`text-center text-2xl tracking-[0.5em] ${
                pinError ? 'border-red-500 focus:ring-red-500' : ''
              }`}
              autoFocus
            />
            {pinError && (
              <p className="text-sm text-red-500 text-center mt-2">
                密码错误，请重试
              </p>
            )}
          </div>
          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              variant="outline"
              onClick={() => setShowPinDialog(false)}
              disabled={isVerifying}
              className="flex-1"
            >
              取消
            </Button>
            <Button
              onClick={handlePinSubmit}
              disabled={pinInput.length < 4 || isVerifying}
              className="flex-1"
            >
              {isVerifying ? '验证中...' : '确认'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
