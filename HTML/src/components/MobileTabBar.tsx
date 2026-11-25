import React from 'react';
import { FlaskConical, ScrollText, Sprout } from 'lucide-react';

export type MainTab = 'bench' | 'workshop' | 'villager';

interface MobileTabBarProps {
  active: MainTab;
  onChange: (tab: MainTab) => void;
  orderCount: number;
}

const TABS: { id: MainTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: 'bench', label: '栽培', icon: Sprout },
  { id: 'workshop', label: '炼金', icon: FlaskConical },
  { id: 'villager', label: '委托', icon: ScrollText },
];

export const MobileTabBar: React.FC<MobileTabBarProps> = ({ active, onChange, orderCount }) => {
  return (
    <nav
      aria-label="主功能导航"
      className="grid shrink-0 grid-cols-3 border-t border-[#3A4A3B] bg-[#212D23] text-[#CFC8B4] lg:hidden"
    >
      {TABS.map(({ id, label, icon: Icon }) => {
        const isActive = active === id;
        return (
          <button
            key={id}
            type="button"
            aria-current={isActive ? 'page' : undefined}
            onClick={() => onChange(id)}
            className={`relative flex h-16 flex-col items-center justify-center gap-1 transition-colors ${
              isActive ? 'text-[#F7DC9C]' : 'text-[#A8B0A0] active:bg-[#2C3A2E]'
            }`}
          >
            <span className="relative">
              <Icon className="h-6 w-6" />
              {id === 'villager' && orderCount > 0 && (
                <span className="absolute -right-2.5 -top-1.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-[#C96F4B] px-1 text-[10px] font-bold text-[#FFFDF8]">
                  {orderCount}
                </span>
              )}
            </span>
            <span className="text-xs font-medium">{label}</span>
            {isActive && <span className="absolute inset-x-6 top-0 h-0.5 rounded-full bg-[#E5C886]" />}
          </button>
        );
      })}
    </nav>
  );
};
