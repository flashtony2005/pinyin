import React from 'react';
import { Home, Play, RotateCcw } from 'lucide-react';
import type { GameState } from '../game/simulation';

interface PauseModalProps {
  state: GameState;
  onResume: () => void;
  onRestart: () => void;
  onReturnTitle: () => void;
}

export const PauseModal: React.FC<PauseModalProps> = ({
  state,
  onResume,
  onRestart,
  onReturnTitle,
}) => {
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="游戏已暂停"
      className="fixed inset-0 z-50 flex items-center justify-center bg-[#141D15]/75 p-4"
    >
      <div className="w-full max-w-md overflow-hidden rounded-2xl border border-[#C7BEA8] bg-[#FAF7EE] shadow-2xl">
        <div className="border-b border-[#DFD8C6] bg-[#243126] px-6 py-4 text-center text-[#FAF5E8]">
          <h2 className="font-display text-2xl font-bold">温室小憩</h2>
          <p className="mt-0.5 text-xs text-[#D5CFC0]">按 Esc 或 P 可随时继续游戏</p>
        </div>

        <div className="p-6">
          {/* 3 compact tabular-nums stat tiles */}
          <div className="mb-6 grid grid-cols-3 gap-3 text-center">
            <div className="rounded-xl border border-[#E0D8C6] bg-[#FFFDF8] p-3">
              <div className="text-[11px] text-[#666C62]">经营天数</div>
              <div className="font-mono-num mt-1 text-xl font-bold text-[#222B23]">
                {state.day}
              </div>
            </div>
            <div className="rounded-xl border border-[#E0D8C6] bg-[#FFFDF8] p-3">
              <div className="text-[11px] text-[#666C62]">完成委托</div>
              <div className="font-mono-num mt-1 text-xl font-bold text-[#2E6F52]">
                {state.ordersCompleted}
              </div>
            </div>
            <div className="rounded-xl border border-[#E0D8C6] bg-[#FFFDF8] p-3">
              <div className="text-[11px] text-[#666C62]">药典收录</div>
              <div className="font-mono-num mt-1 text-xl font-bold text-[#C96F4B]">
                {state.discoveredSyllables.length}
              </div>
            </div>
          </div>

          <div className="flex flex-col gap-2.5">
            <button
              type="button"
              onClick={onResume}
              className="inline-flex h-11 w-full items-center justify-center gap-2 rounded-xl bg-[#2E6F52] text-sm font-semibold text-[#FFFDF8] hover:bg-[#235840]"
            >
              <Play className="h-4 w-4 fill-current" />
              <span>继续经营</span>
            </button>

            <button
              type="button"
              onClick={onRestart}
              className="inline-flex h-11 w-full items-center justify-center gap-2 rounded-xl border border-[#D4CCB9] bg-[#FFFDF8] text-sm font-semibold text-[#2B342C] hover:bg-[#F2EDE0]"
            >
              <RotateCcw className="h-4 w-4" />
              <span>重新开始新温室</span>
            </button>

            <button
              type="button"
              onClick={onReturnTitle}
              className="inline-flex h-10 w-full items-center justify-center gap-2 rounded-xl text-xs font-medium text-[#60685F] hover:bg-[#EDE7D8] hover:text-[#232B23]"
            >
              <Home className="h-4 w-4" />
              <span>返回标题画面</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
