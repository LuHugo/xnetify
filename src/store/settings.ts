import { SettingsSidebarItem } from '@/types';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
interface SettingsState {
    open: boolean
    activeItem: SettingsSidebarItem
    setOpen: (open: boolean) => void
    setActiveItem: (item: SettingsSidebarItem) => void
}
export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      open: false,
      activeItem: "general",
      setOpen: (open) => set({ open, activeItem: "general" }),
      setActiveItem: (item) => set({ activeItem: item }),
    }),
    {
      name: 'settings',
      partialize: (state) => ({
        open: state.open,
        activeItem: state.activeItem
      }),
    }
  )
);