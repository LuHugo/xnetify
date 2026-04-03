import { useState } from 'react';
import { useAppStore, TimeSlot } from '../store';
import { Card, CardContent, CardHeader, CardTitle } from './ui/card';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Clock, Plus, Trash2, Save, AlertCircle } from 'lucide-react';

const DAYS = [
  { value: 0, label: '日' },
  { value: 1, label: '一' },
  { value: 2, label: '二' },
  { value: 3, label: '三' },
  { value: 4, label: '四' },
  { value: 5, label: '五' },
  { value: 6, label: '六' },
];

export function TimeRules() {
  const { timeRule, updateTimeRule, mode } = useAppStore();
  const [slots, setSlots] = useState<TimeSlot[]>(timeRule.slots);
  const [enabled, setEnabled] = useState(timeRule.enabled);
  const [maxMinutes, setMaxMinutes] = useState(timeRule.max_daily_minutes);
  const [hasChanges, setHasChanges] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const addSlot = () => {
    const newSlot: TimeSlot = {
      start: '08:00',
      end: '22:00',
      days: [1, 2, 3, 4, 5],
    };
    setSlots([...slots, newSlot]);
    setHasChanges(true);
  };

  const updateSlot = (index: number, updates: Partial<TimeSlot>) => {
    const newSlots = [...slots];
    newSlots[index] = { ...newSlots[index], ...updates };
    setSlots(newSlots);
    setHasChanges(true);
  };

  const removeSlot = (index: number) => {
    setSlots(slots.filter((_, i) => i !== index));
    setHasChanges(true);
  };

  const toggleDay = (slotIndex: number, day: number) => {
    const slot = slots[slotIndex];
    const newDays = slot.days.includes(day)
      ? slot.days.filter((d) => d !== day)
      : [...slot.days, day].sort();
    updateSlot(slotIndex, { days: newDays });
  };

  const saveChanges = async () => {
    setIsSaving(true);
    try {
      await updateTimeRule({
        enabled,
        slots,
        max_daily_minutes: maxMinutes,
      });
      setHasChanges(false);
    } finally {
      setIsSaving(false);
    }
  };

  const isReadOnly = mode === 'teen';

  return (
    <Card className="bg-slate-900/50 border-slate-800 backdrop-blur-sm">
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2 text-white">
          <Clock className="h-5 w-5 text-amber-400" />
          成长时光设置
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between">
          <label className="flex items-center gap-3 cursor-pointer">
            <div className="relative">
              <input
                type="checkbox"
                id="enable-time-rule"
                checked={enabled}
                onChange={(e) => {
                  setEnabled(e.target.checked);
                  setHasChanges(true);
                }}
                disabled={isReadOnly}
                className="sr-only"
              />
              <div className={`w-12 h-7 rounded-full transition-colors ${
                enabled ? 'bg-emerald-500' : 'bg-slate-700'
              }`}>
                <div className={`w-5 h-5 bg-white rounded-full shadow-md transform transition-transform mt-1 ${
                  enabled ? 'translate-x-6 ml-1' : 'translate-x-1'
                }`} />
              </div>
            </div>
            <span className="text-slate-300">启用成长时光管理</span>
          </label>
          {!isReadOnly && hasChanges && (
            <Button 
              size="sm" 
              onClick={saveChanges}
              disabled={isSaving}
              className="bg-emerald-500 hover:bg-emerald-600 text-white gap-2"
            >
              <Save className="h-4 w-4" />
              {isSaving ? '保存中...' : '保存'}
            </Button>
          )}
        </div>

        {enabled && (
          <>
            <div className="p-4 rounded-xl bg-slate-800/50 space-y-3">
              <div className="flex items-center justify-between">
                <label className="text-sm text-slate-400">
                  每日成长时光上限
                </label>
                {isReadOnly && (
                  <span className="flex items-center gap-1 text-xs text-amber-400">
                    <AlertCircle className="h-3 w-3" />
                    仅家长可修改
                  </span>
                )}
              </div>
              <div className="flex items-center gap-4">
                <Input
                  type="number"
                  min={30}
                  max={1440}
                  step={30}
                  value={maxMinutes}
                  onChange={(e) => {
                    setMaxMinutes(parseInt(e.target.value) || 480);
                    setHasChanges(true);
                  }}
                  disabled={isReadOnly}
                  className="w-28 bg-slate-900 border-slate-700 text-white text-center"
                />
                <span className="text-slate-400">分钟</span>
                <span className="text-sm text-emerald-400">
                  ({Math.floor(maxMinutes / 60)} 小时 {maxMinutes % 60} 分钟)
                </span>
              </div>
            </div>

            <div className="space-y-3">
              {slots.length === 0 ? (
                <div className="p-6 text-center text-slate-500">
                  暂未设置成长时段
                </div>
              ) : (
                slots.map((slot, index) => (
                  <div
                    key={index}
                    className="p-4 rounded-xl bg-slate-800/50 space-y-3"
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium text-white">
                        时段 {index + 1}
                      </span>
                      {!isReadOnly && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => removeSlot(index)}
                          className="text-slate-400 hover:text-rose-400"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>

                    <div className="flex items-center gap-4">
                      <div className="flex-1 space-y-1">
                        <label className="text-xs text-slate-500">开始</label>
                        <Input
                          type="time"
                          value={slot.start}
                          onChange={(e) =>
                            updateSlot(index, { start: e.target.value })
                          }
                          disabled={isReadOnly}
                          className="bg-slate-900 border-slate-700 text-white"
                        />
                      </div>
                      <div className="flex-1 space-y-1">
                        <label className="text-xs text-slate-500">结束</label>
                        <Input
                          type="time"
                          value={slot.end}
                          onChange={(e) =>
                            updateSlot(index, { end: e.target.value })
                          }
                          disabled={isReadOnly}
                          className="bg-slate-900 border-slate-700 text-white"
                        />
                      </div>
                    </div>

                    <div className="space-y-2">
                      <label className="text-xs text-slate-500">适用日期</label>
                      <div className="flex gap-2">
                        {DAYS.map((day) => (
                          <button
                            key={day.value}
                            type="button"
                            onClick={() => toggleDay(index, day.value)}
                            disabled={isReadOnly}
                            className={`w-10 h-10 rounded-lg text-sm font-medium transition-colors ${
                              slot.days.includes(day.value)
                                ? 'bg-emerald-500 text-white'
                                : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
                            } disabled:opacity-50`}
                          >
                            {day.label}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                ))
              )}

              {!isReadOnly && (
                <Button 
                  variant="outline" 
                  onClick={addSlot} 
                  className="w-full border-dashed border-slate-700 text-slate-400 hover:text-white hover:border-slate-500"
                >
                  <Plus className="h-4 w-4 mr-2" />
                  添加时段
                </Button>
              )}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
