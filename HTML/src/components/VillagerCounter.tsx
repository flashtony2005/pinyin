import React, { useState } from 'react';
import { CheckCircle2, Coins, HelpCircle, Send, Volume2 } from 'lucide-react';
import type { ToneNumber } from '../data/pinyinResources';
import type { BrewedPotion, VillagerOrder } from '../game/simulation';

interface VillagerCounterProps {
  orders: VillagerOrder[];
  potions: BrewedPotion[];
  hasEchoBell: boolean;
  onListenOrder: (syllable: string, tone: ToneNumber, slow: boolean) => void;
  onRevealHint: (orderId: string) => void;
  onFulfillOrder: (orderId: string) => void;
}

const MODE_LABELS: Record<VillagerOrder['mode'], string> = {
  listen: '听音配药',
  character: '看字拼读',
  tone_rule: '规则挑战',
};

export const VillagerCounter: React.FC<VillagerCounterProps> = ({
  orders,
  potions,
  hasEchoBell,
  onListenOrder,
  onRevealHint,
  onFulfillOrder,
}) => {
  const [slowMode, setSlowMode] = useState(false);

  return (
    <section
      aria-label="森林村民委托柜台"
      className="flex h-full flex-col overflow-hidden rounded-2xl border border-[#CFC6B2] bg-[#FAF7EE] shadow-sm"
    >
      {/* Header */}
      <div className="flex items-center justify-between gap-2 border-b border-[#DFD8C6] bg-[#364438] px-4 py-3 text-[#FAF5E8]">
        <div className="min-w-0">
          <h2 className="font-display text-lg font-bold tracking-wide">村民委托柜台</h2>
          <p className="truncate text-xs text-[#D6D2C4]">听音或看字拼读，交付药水赚金币</p>
        </div>
        <button
          type="button"
          onClick={() => setSlowMode((v) => !v)}
          aria-pressed={slowMode}
          className={`h-8 rounded-lg border px-2.5 text-xs font-medium transition-colors whitespace-nowrap shrink-0 ${
            slowMode
              ? 'border-[#E5C886] bg-[#C96F4B] text-[#FFFDF8]'
              : 'border-[#5B6E5D] bg-[#28342A] text-[#E6DFCE] hover:bg-[#334335]'
          }`}
        >
          {slowMode ? '慢速：开' : '慢速：关'}
        </button>
      </div>

      {/* 3 Villager Order Cards — scrollable on phones so every card (and its
          listen / deliver buttons) stays reachable in a short viewport */}
      <div className="flex min-h-0 flex-1 flex-col divide-y divide-[#E2DAC8] overflow-y-auto">
        {orders.map((order, idx) => {
          const requiredPotionId = `${order.targetSyllable}:${order.targetTone}`;
          const ready = potions.some((p) => p.id === requiredPotionId && p.count > 0);
          const showPinyin = order.mode !== 'listen' || order.hintRevealed;

          return (
            <article
              key={order.id}
              data-testid={`order-card-${idx}`}
              className={`flex flex-col gap-2 px-3.5 py-3 transition-colors ${
                ready ? 'bg-[#F0F7F1]' : 'bg-[#FAF7EE]'
              }`}
            >
              {/* Villager Portrait & Identity */}
              <div className="flex items-start gap-2.5">
                <div className="h-12 w-12 shrink-0 overflow-hidden rounded-xl border border-[#CFC5B0] bg-[#EFE8D6]">
                  <img
                    src={order.avatar}
                    alt={`${order.villagerName}肖像`}
                    referrerPolicy="no-referrer"
                    className="h-full w-full object-cover"
                  />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center justify-between gap-2">
                    <div className="truncate text-sm font-bold text-[#222B23]">
                      {order.villagerName}
                    </div>
                    <div className="font-mono-num flex shrink-0 items-center gap-1 text-xs font-semibold text-[#9E6A14]">
                      <Coins className="h-3.5 w-3.5" />
                      <span>+{order.rewardGold}</span>
                    </div>
                  </div>

                  {/* Unboxed typographic metadata */}
                  <div className="mt-0.5 flex items-center gap-1.5 text-[11px] text-[#646B61]">
                    <span>{order.villagerRole}</span>
                    <span aria-hidden="true">·</span>
                    <span>{MODE_LABELS[order.mode]}</span>
                    {ready && (
                      <>
                        <span aria-hidden="true">·</span>
                        <span className="inline-flex items-center gap-0.5 font-semibold text-[#236B48]">
                          <CheckCircle2 className="h-3 w-3" />
                          可交付
                        </span>
                      </>
                    )}
                  </div>

                  <p className="mt-1 text-xs leading-snug text-[#2E372F]">
                    {order.promptText}
                  </p>
                </div>
              </div>

              {/* Clue & Audio Row */}
              <div className="flex flex-wrap items-center justify-between gap-1.5 rounded-xl border border-[#DFD7C5] bg-[#FFFDF8] px-2.5 py-1.5">
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() =>
                      onListenOrder(order.targetSyllable, order.targetTone, slowMode)
                    }
                    aria-label={`听 ${order.villagerName} 的目标读音`}
                    className="inline-flex h-8 items-center gap-1 rounded-lg bg-[#2E6F52] px-2.5 text-xs font-semibold text-[#FFFDF8] transition-colors hover:bg-[#235840] whitespace-nowrap shrink-0"
                  >
                    <Volume2 className="h-3.5 w-3.5" />
                    <span>{order.mode === 'listen' ? '听音' : '听读音'}</span>
                  </button>

                  {showPinyin ? (
                    <div className="text-xs text-[#3D463E]">
                      <span className="font-display text-sm font-bold text-[#1F583D]">
                        {order.targetMarked}
                      </span>
                      <span className="ml-1 text-[11px] text-[#666C62]">
                        (
                        {order.targetMedial
                          ? `${order.targetInitial}+${order.targetMedial}+${order.targetFinal}`
                          : `${order.targetInitial || '零'}+${order.targetFinal}`}
                        ·{order.targetTone}声)
                      </span>
                    </div>
                  ) : (
                    <span className="text-[11px] text-[#6B7166]">听声音辨声韵与声调</span>
                  )}
                </div>

                {order.mode === 'listen' && !order.hintRevealed && (
                  <button
                    type="button"
                    onClick={() => onRevealHint(order.id)}
                    className="inline-flex h-7 items-center gap-1 rounded-lg border border-[#D8CEBA] bg-[#F6F1E5] px-2 text-[11px] font-medium text-[#5A5344] hover:bg-[#ECE4D3] whitespace-nowrap shrink-0"
                  >
                    <HelpCircle className="h-3 w-3" />
                    <span>{hasEchoBell ? '免费提示' : '提示 5金'}</span>
                  </button>
                )}
              </div>

              {/* Deliver Action Button */}
              <button
                type="button"
                data-testid={`deliver-order-${idx}`}
                disabled={!ready}
                onClick={() => onFulfillOrder(order.id)}
                className={`inline-flex h-9 w-full items-center justify-center gap-1.5 rounded-xl text-xs font-semibold transition-colors ${
                  ready
                    ? 'bg-[#C96F4B] text-[#FFFDF8] shadow-xs hover:bg-[#B45D3B]'
                    : 'cursor-not-allowed border border-[#DDD5C4] bg-[#F2EDE1] text-[#7D7A70]'
                }`}
              >
                <Send className="h-3.5 w-3.5" />
                <span className="truncate">
                  {ready
                    ? `交付「${order.targetMarked}（${order.targetChar}）」药水`
                    : `待炼制：${showPinyin ? `${order.targetMarked}（${order.targetChar}）` : '请先听音并在炼金台调配'}`}
                </span>
              </button>
            </article>
          );
        })}
      </div>
    </section>
  );
};
