import React from 'react';
import {
  BookOpen,
  Coins,
  FileText,
  Moon,
  Pause,
  Sparkles,
  Store,
  Volume2,
  VolumeX,
} from 'lucide-react';
import type { GameState } from '../game/simulation';
import { getSeasonProgress } from '../game/simulation';
import { TOTAL_DISCOVERABLE_SYLLABLES } from '../game/pinyinEngine';

interface TopHudProps {
  state: GameState;
  muted: boolean;
  onAdvanceDay: () => void;
  onOpenShop: () => void;
  onOpenGrimoire: () => void;
  onOpenAnalysis: () => void;
  onToggleMute: () => void;
  onPause: () => void;
}

export const TopHud: React.FC<TopHudProps> = ({
  state,
  muted,
  onAdvanceDay,
  onOpenShop,
  onOpenGrimoire,
  onOpenAnalysis,
  onToggleMute,
  onPause,
}) => {
  const { current, next, ordersToNext } = getSeasonProgress(state);
  const totalSyllables = TOTAL_DISCOVERABLE_SYLLABLES;

  return (
    <div className="shrink-0 w-full border-b border-[#405241] bg-[#212D23] text-[#F5EFE0] shadow-md">
      <div className="mx-auto flex max-w-[1420px] flex-wrap items-center justify-between gap-x-3 gap-y-2 px-3 py-2 sm:px-6 sm:py-2.5">
        {/* Left Zone: Season, Day & Energy Vitality Sockets */}
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 sm:gap-6">
          <div className="flex items-center gap-3">
            <svg viewBox="0 0 28 28" className="h-7 w-7 shrink-0" aria-hidden="true">
              <circle cx="14" cy="14" r="12" fill="#2D4030" stroke="#D8B36A" strokeWidth="1.6" />
              <path
                d="M14 20 V9 M14 14 C9 12 8 8 13 7 C14 10 14 12 14 14 Z M14 12 C19 10 20 7 15 6 C14 9 14 10 14 12 Z"
                fill="#8BC37A"
              />
            </svg>
            <div>
              <div className="flex items-baseline gap-2">
                <span className="font-display text-base font-semibold tracking-wide text-[#FAF4E3] whitespace-nowrap">
                  第 <span className="font-mono-num">{state.day}</span> 天
                </span>
                <span className="hidden text-xs text-[#D8C9A8]/80 sm:inline">·</span>
                <span className="hidden text-sm font-medium text-[#E7C986] whitespace-nowrap sm:inline">
                  {current.name}
                </span>
              </div>
              <div className="hidden text-[11px] text-[#C8C3B4] sm:block">
                {next
                  ? `再完成 ${ordersToNext} 份村民委托解锁「${next.name.split(' · ')[0]}」`
                  : '已解锁四季全部声母与韵母种子'}
              </div>
            </div>
          </div>

          {/* Energy Pips — the full pip row is desktop-only; phones show a
              single drop + number to keep the HUD to two rows */}
          <div
            className="flex items-center gap-2 border-l border-[#3A4A3B] pl-2.5 sm:gap-2.5 sm:pl-4"
            aria-label={`今日精力 ${state.energy} / ${state.maxEnergy}`}
          >
            <div>
              <div className="text-[10px] font-medium tracking-wider text-[#C6BFA9]">
                精力
              </div>
              <div className="mt-0.5 flex items-center gap-1">
                <svg viewBox="0 0 14 16" className="h-4 w-3.5 sm:hidden" aria-hidden="true">
                  <path
                    d="M7 1.2 C7 1.2 12.2 6.8 12.2 10.2 C12.2 13.1 9.9 15 7 15 C4.1 15 1.8 13.1 1.8 10.2 C1.8 6.8 7 1.2 7 1.2 Z"
                    fill="#6BC48B"
                    stroke="#C8F0D4"
                    strokeWidth="1.4"
                  />
                </svg>
                <span className="hidden gap-1 sm:flex">
                  {Array.from({ length: state.maxEnergy }).map((_, i) => {
                    const full = i < state.energy;
                    return (
                      <svg
                        key={i}
                        viewBox="0 0 14 16"
                        className="h-4 w-3.5"
                        aria-hidden="true"
                      >
                        <path
                          d="M7 1.2 C7 1.2 12.2 6.8 12.2 10.2 C12.2 13.1 9.9 15 7 15 C4.1 15 1.8 13.1 1.8 10.2 C1.8 6.8 7 1.2 7 1.2 Z"
                          fill={full ? '#6BC48B' : 'transparent'}
                          stroke={full ? '#C8F0D4' : '#5F7362'}
                          strokeWidth="1.4"
                        />
                      </svg>
                    );
                  })}
                </span>
                <span className="font-mono-num ml-0.5 text-sm font-semibold text-[#FAF4E3] sm:ml-1">
                  {state.energy}/{state.maxEnergy}
                </span>
              </div>
            </div>
          </div>

          {/* Coins & Grimoire Discovery Counters */}
          <div className="flex items-center gap-3 border-l border-[#3A4A3B] pl-2.5 sm:gap-5 sm:pl-4">
            <div className="flex items-center gap-2">
              <Coins className="h-4 w-4 text-[#E6B85C]" aria-hidden="true" />
              <div>
                <div className="text-[10px] tracking-wider text-[#C6BFA9]">星尘金币</div>
                <div
                  data-testid="hud-coins"
                  className="font-mono-num text-base font-semibold leading-tight text-[#F7DC9C]"
                >
                  {state.coins}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Sparkles className="h-4 w-4 text-[#9AC98D]" aria-hidden="true" />
              <div>
                <div className="text-[10px] tracking-wider text-[#C6BFA9]">药典收录</div>
                <div
                  data-testid="hud-discovered"
                  className="font-mono-num text-base font-semibold leading-tight text-[#E3F0DF]"
                >
                  {state.discoveredSyllables.length}/{totalSyllables}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Right Zone: Core Sim Actions + Compact Utility Buttons */}
        <div className="flex flex-wrap items-center gap-1.5 sm:gap-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              onAdvanceDay();
            }}
            className="inline-flex h-10 items-center gap-1.5 rounded-lg border border-[#E5C886]/50 bg-[#C96F4B] px-2.5 text-sm font-semibold text-[#FFFDF8] transition-colors hover:bg-[#B85E3A] focus-visible:outline-2 focus-visible:outline-[#E5C886] whitespace-nowrap shrink-0 sm:px-3.5"
          >
            <Moon className="h-4 w-4" />
            <span>结束今天</span>
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              onOpenShop();
            }}
            className="inline-flex h-10 items-center gap-1.5 rounded-lg border border-[#526554] bg-[#2D3B2F] px-3 text-sm font-medium text-[#F4ECDC] transition-colors hover:border-[#D8B36A]/60 hover:bg-[#364738] focus-visible:outline-2 focus-visible:outline-[#E5C886] whitespace-nowrap shrink-0"
          >
            <Store className="h-4 w-4 text-[#E5C886]" />
            <span className="hidden sm:inline">温室商店</span>
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              onOpenGrimoire();
            }}
            className="inline-flex h-10 items-center gap-1.5 rounded-lg border border-[#526554] bg-[#2D3B2F] px-3 text-sm font-medium text-[#F4ECDC] transition-colors hover:border-[#D8B36A]/60 hover:bg-[#364738] focus-visible:outline-2 focus-visible:outline-[#E5C886] whitespace-nowrap shrink-0"
          >
            <BookOpen className="h-4 w-4 text-[#9AC98D]" />
            <span className="hidden sm:inline">拼音药典</span>
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              onOpenAnalysis();
            }}
            aria-label="策划与资源复用分析"
            title="策划与资源复用分析"
            className="inline-flex h-10 items-center gap-1.5 rounded-lg border border-[#526554] bg-[#2D3B2F] px-3 text-sm font-medium text-[#DFD8C6] transition-colors hover:border-[#D8B36A]/60 hover:bg-[#364738] focus-visible:outline-2 focus-visible:outline-[#E5C886] whitespace-nowrap shrink-0"
          >
            <FileText className="h-4 w-4 text-[#D8B36A]" />
            <span className="hidden xl:inline">策划分析</span>
          </button>

          <button
            type="button"
            tabIndex={-1}
            onClick={(e) => {
              e.currentTarget.blur();
              onToggleMute();
            }}
            aria-label={muted ? '开启声音 [M]' : '静音 [M]'}
            title={muted ? '开启声音 [M]' : '静音 [M]'}
            className="flex h-10 w-10 items-center justify-center rounded-lg border border-[#526554] bg-[#2A372C] text-[#EFE6D2] transition-colors hover:border-[#D8B36A]/60 hover:bg-[#364738]"
          >
            {muted ? <VolumeX className="h-4 w-4" /> : <Volume2 className="h-4 w-4" />}
          </button>

          <button
            type="button"
            tabIndex={-1}
            onClick={(e) => {
              e.currentTarget.blur();
              onPause();
            }}
            aria-label="暂停 [P]"
            title="暂停 [P]"
            className="flex h-10 w-10 items-center justify-center rounded-lg border border-[#526554] bg-[#2A372C] text-[#EFE6D2] transition-colors hover:border-[#D8B36A]/60 hover:bg-[#364738]"
          >
            <Pause className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
};
