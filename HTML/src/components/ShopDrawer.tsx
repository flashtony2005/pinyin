import React, { useEffect, useState } from 'react';
import {
  Check,
  Coins,
  Flower2,
  Lock,
  Palette,
  Plus,
  ShoppingBag,
  Sparkles,
  Sprout,
  X,
} from 'lucide-react';
import {
  DECOR_CATALOG,
  POT_THEMES,
  SEASONS,
  type DecorId,
  type PotThemeId,
} from '../data/pinyinResources';
import {
  MAX_PLOTS,
  SEED_PACK_COST,
  SEED_PACK_YIELD,
  getNextPlotCost,
  type GameState,
  type SeedKind,
} from '../game/simulation';

export type ShopTabId = 'pots' | 'seeds' | 'decors';

interface ShopDrawerProps {
  state: GameState;
  initialTab?: ShopTabId;
  onClose: () => void;
  onBuyDecor: (id: DecorId) => void;
  onSelectTheme: (id: PotThemeId) => void;
  onUnlockSeason: (seasonIndex: number) => void;
  onExpandPlot: () => void;
  onBuySeedPack: (seedId: string, kind: SeedKind) => void;
  onQuickPlant: (seedId: string, kind: SeedKind) => void;
}

export const ShopDrawer: React.FC<ShopDrawerProps> = ({
  state,
  initialTab = 'pots',
  onClose,
  onBuyDecor,
  onSelectTheme,
  onUnlockSeason,
  onExpandPlot,
  onBuySeedPack,
  onQuickPlant,
}) => {
  const [tab, setTab] = useState<ShopTabId>(initialTab);
  const [seedKindFilter, setSeedKindFilter] = useState<SeedKind>('initial');

  useEffect(() => {
    setTab(initialTab);
  }, [initialTab]);

  const nextPlotCost = getNextPlotCost(state.plots.length);
  const canExpand = nextPlotCost !== null && state.coins >= nextPlotCost;
  const hasEmptyPot = state.plots.some((p) => p.seedId === null);
  const visibleSeeds =
    seedKindFilter === 'initial' ? state.unlockedInitials : state.unlockedFinals;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="温室工坊商店"
      className="fixed inset-0 z-40 flex items-center justify-center bg-[#141D15]/70 p-4"
    >
      <div className="flex max-h-[88vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-[#C7BEA8] bg-[#FAF7EE] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#DFD8C6] bg-[#243126] px-6 py-4 text-[#FAF5E8]">
          <div>
            <h2 className="font-display text-xl font-bold">温室工坊商店</h2>
            <p className="text-xs text-[#D5CFC0]">
              购买新花盆扩建花架、补货声母韵母种子素材，或添置功能性魔法装饰
            </p>
          </div>
          <div className="flex items-center gap-4">
            <div
              data-testid="shop-coins"
              className="font-mono-num flex items-center gap-1.5 text-sm font-bold text-[#F5D68B]"
            >
              <Coins className="h-4 w-4" />
              <span>{state.coins}</span>
            </div>
            <button
              type="button"
              onClick={onClose}
              aria-label="关闭商店"
              className="flex h-9 w-9 items-center justify-center rounded-lg border border-[#4D604F] bg-[#304032] text-[#EFE8D8] hover:bg-[#3B4F3E]"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* 3 Clear Shop Tabs */}
        <div className="flex flex-wrap items-center gap-2 border-b border-[#E3DCCB] bg-[#F2EDE0] px-6 py-2.5">
          <button
            type="button"
            onClick={() => setTab('pots')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold transition-colors whitespace-nowrap ${
              tab === 'pots'
                ? 'bg-[#2E6F52] text-[#FFFDF8]'
                : 'text-[#4E564E] hover:bg-[#E5DEC9]'
            }`}
          >
            <Flower2 className="h-3.5 w-3.5" />
            <span>花盆与扩建 ({state.plots.length}/{MAX_PLOTS})</span>
          </button>

          <button
            type="button"
            onClick={() => setTab('seeds')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold transition-colors whitespace-nowrap ${
              tab === 'seeds'
                ? 'bg-[#2E6F52] text-[#FFFDF8]'
                : 'text-[#4E564E] hover:bg-[#E5DEC9]'
            }`}
          >
            <Sprout className="h-3.5 w-3.5" />
            <span>种子与素材铺</span>
          </button>

          <button
            type="button"
            onClick={() => setTab('decors')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-2 text-xs font-semibold transition-colors whitespace-nowrap ${
              tab === 'decors'
                ? 'bg-[#2E6F52] text-[#FFFDF8]'
                : 'text-[#4E564E] hover:bg-[#E5DEC9]'
            }`}
          >
            <Palette className="h-3.5 w-3.5" />
            <span>魔法装饰</span>
          </button>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 space-y-6 overflow-y-auto p-6">
          {tab === 'pots' && (
            <>
              {/* Buy Extra Flower Pot Card */}
              <div className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-[#C6D5C9] bg-[#EFF6F1] p-4">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="text-sm font-bold text-[#1E5239]">购买新花盆 · 扩建栽培花架</h3>
                    <span className="font-mono-num text-xs font-semibold text-[#35634B]">
                      当前 {state.plots.length} / {MAX_PLOTS} 盆
                    </span>
                  </div>
                  <p className="mt-1 text-xs text-[#3C5042]">
                    {nextPlotCost !== null
                      ? `每购买 1 个新花盆，温室花架就会增加 1 个独立种植位（最多 ${MAX_PLOTS} 盆），可同时培育更多声母与韵母。`
                      : `已扩建至最大规模（${MAX_PLOTS} 个花盆位），温室花架已全部铺满！`}
                  </p>
                </div>

                <button
                  type="button"
                  data-testid="shop-expand-plot-btn"
                  disabled={!canExpand}
                  onClick={onExpandPlot}
                  className={`inline-flex h-10 shrink-0 items-center gap-1.5 rounded-xl px-4 text-xs font-semibold transition-colors whitespace-nowrap ${
                    nextPlotCost === null
                      ? 'bg-[#DCECE1] text-[#225C40]'
                      : canExpand
                        ? 'bg-[#C96F4B] text-[#FFFDF8] hover:bg-[#B45D3B]'
                        : 'cursor-not-allowed bg-[#E5E0D2] text-[#7F7B6F]'
                  }`}
                >
                  {nextPlotCost === null ? (
                    <>
                      <Check className="h-4 w-4" />
                      <span>已达上限</span>
                    </>
                  ) : (
                    <>
                      <Plus className="h-4 w-4" />
                      <span>买 1 个新花盆（{nextPlotCost} 金）</span>
                    </>
                  )}
                </button>
              </div>

              {/* Pot Glaze Themes */}
              <div>
                <h3 className="mb-3 text-sm font-bold text-[#253026]">温室花盆釉色外观</h3>
                <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                  {POT_THEMES.map((theme) => {
                    const unlocked = state.unlockedThemes.includes(theme.id);
                    const active = state.potTheme === theme.id;
                    const affordable = state.coins >= theme.cost;

                    return (
                      <button
                        key={theme.id}
                        type="button"
                        disabled={!unlocked && !affordable}
                        onClick={() => onSelectTheme(theme.id)}
                        className={`flex flex-col items-center rounded-xl border p-3.5 text-center transition-colors ${
                          active
                            ? 'border-[#2E6F52] bg-[#EAF4ED]'
                            : unlocked || affordable
                              ? 'border-[#DDD5C3] bg-[#FFFDF8] hover:border-[#ABA28E]'
                              : 'cursor-not-allowed border-[#E5E0D4] bg-[#F5F2EA] opacity-65'
                        }`}
                      >
                        <span
                          className="mb-2 h-8 w-10 rounded-b-lg border-t-4"
                          style={{
                            backgroundColor: theme.bodyHex,
                            borderTopColor: theme.rimHex,
                          }}
                        />
                        <span className="text-xs font-semibold text-[#232B23]">{theme.name}</span>
                        <span className="mt-0.5 text-[11px] text-[#666C62]">
                          {active ? '使用中' : unlocked ? '点击切换' : `${theme.cost} 金解锁`}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            </>
          )}

          {tab === 'seeds' && (
            <>
              {/* Direct Seed / Essence Replenishment Market */}
              <div>
                <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <h3 className="text-sm font-bold text-[#253026]">
                      声韵种子与素材直购（{SEED_PACK_COST} 金币 / +{SEED_PACK_YIELD} 库存）
                    </h3>
                    <p className="text-xs text-[#5E665C]">
                      蒸馏台缺货时可直接买种子补货，或一键种入温室空花盆免费持续收获
                    </p>
                  </div>

                  <div className="flex items-center gap-1 rounded-lg bg-[#E6E0CE] p-1">
                    <button
                      type="button"
                      onClick={() => setSeedKindFilter('initial')}
                      className={`rounded-md px-3 py-1 text-xs font-semibold transition-colors whitespace-nowrap ${
                        seedKindFilter === 'initial'
                          ? 'bg-[#FFFDF8] text-[#1F583D] shadow-xs'
                          : 'text-[#5A6258] hover:text-[#232B23]'
                      }`}
                    >
                      声母 ({state.unlockedInitials.length})
                    </button>
                    <button
                      type="button"
                      onClick={() => setSeedKindFilter('final')}
                      className={`rounded-md px-3 py-1 text-xs font-semibold transition-colors whitespace-nowrap ${
                        seedKindFilter === 'final'
                          ? 'bg-[#FFFDF8] text-[#A24B24] shadow-xs'
                          : 'text-[#5A6258] hover:text-[#232B23]'
                      }`}
                    >
                      韵母 ({state.unlockedFinals.length})
                    </button>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
                  {visibleSeeds.map((seed) => {
                    const stock = state.pantry[seed] ?? 0;
                    const canBuy = state.coins >= SEED_PACK_COST;
                    const isPlanted = state.plots.some((p) => p.seedId === seed);

                    return (
                      <div
                        key={seed}
                        className={`flex flex-col justify-between rounded-xl border p-3 ${
                          stock === 0
                            ? 'border-[#DFC2B5] bg-[#FDF6F2]'
                            : 'border-[#DDD5C3] bg-[#FFFDF8]'
                        }`}
                      >
                        <div className="flex items-center justify-between gap-2">
                          <div className="flex items-baseline gap-2">
                            <span
                              className={`font-display text-lg font-bold ${
                                seedKindFilter === 'initial'
                                  ? 'text-[#1F583D]'
                                  : 'text-[#A24B24]'
                              }`}
                            >
                              {seed}
                            </span>
                            <span
                              className={`font-mono-num text-xs font-semibold ${
                                stock === 0 ? 'text-[#B8473E]' : 'text-[#5E665C]'
                              }`}
                            >
                              库存 {stock}
                            </span>
                          </div>
                          {isPlanted && (
                            <span className="text-[11px] font-medium text-[#2E6F52]">已种植</span>
                          )}
                        </div>

                        <div className="mt-2.5 flex items-center gap-1.5">
                          <button
                            type="button"
                            data-testid={`buy-seed-${seed}`}
                            disabled={!canBuy}
                            onClick={() => onBuySeedPack(seed, seedKindFilter)}
                            className={`inline-flex h-8 flex-1 items-center justify-center gap-1 rounded-lg px-2 text-xs font-semibold transition-colors whitespace-nowrap ${
                              canBuy
                                ? 'bg-[#C96F4B] text-[#FFFDF8] hover:bg-[#B45D3B]'
                                : 'cursor-not-allowed bg-[#EAE5D8] text-[#8A8578]'
                            }`}
                          >
                            <ShoppingBag className="h-3.5 w-3.5" />
                            <span>+{SEED_PACK_YIELD} ({SEED_PACK_COST}金)</span>
                          </button>

                          <button
                            type="button"
                            disabled={!hasEmptyPot}
                            onClick={() => onQuickPlant(seed, seedKindFilter)}
                            title={
                              hasEmptyPot
                                ? `免费种入空花盆培育「${seed}」`
                                : '花架暂无空盆，可先去「花盆与扩建」买新花盆'
                            }
                            className={`inline-flex h-8 items-center justify-center rounded-lg border px-2 text-[11px] font-semibold transition-colors whitespace-nowrap ${
                              hasEmptyPot
                                ? 'border-[#B9CEBF] bg-[#EEF6F0] text-[#1F583D] hover:bg-[#DFEFE4]'
                                : 'cursor-not-allowed border-[#E2DDD0] bg-[#F5F2EA] text-[#9A968B]'
                            }`}
                          >
                            种盆
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Seasonal Seed Unlocks */}
              <div className="border-t border-[#E2DAC8] pt-5">
                <h3 className="mb-3 text-sm font-bold text-[#253026]">四季新品种种子包</h3>
                <div className="space-y-3">
                  {SEASONS.map((season, index) => {
                    const unlocked = index <= state.seasonIndex;
                    const earlyCost = 30 * index;
                    const canBuyEarly = !unlocked && state.coins >= earlyCost;

                    return (
                      <div
                        key={season.id}
                        className={`rounded-xl border p-4 ${
                          unlocked
                            ? 'border-[#B9D3C2] bg-[#F0F7F2]'
                            : 'border-[#DDD5C3] bg-[#FFFDF8]'
                        }`}
                      >
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div>
                            <div className="font-semibold text-[#222B23]">{season.name}</div>
                            <p className="mt-0.5 text-xs text-[#5B6359]">{season.subtitle}</p>
                          </div>
                          {unlocked ? (
                            <span className="inline-flex items-center gap-1 text-xs font-semibold text-[#246848]">
                              <Check className="h-4 w-4" />
                              已解锁
                            </span>
                          ) : (
                            <button
                              type="button"
                              disabled={!canBuyEarly}
                              onClick={() => onUnlockSeason(index)}
                              className={`inline-flex h-9 items-center gap-1.5 rounded-lg px-3.5 text-xs font-semibold transition-colors whitespace-nowrap ${
                                canBuyEarly
                                  ? 'bg-[#C96F4B] text-[#FFFDF8] hover:bg-[#B45D3B]'
                                  : 'cursor-not-allowed bg-[#EAE5D8] text-[#7D786C]'
                              }`}
                            >
                              <Lock className="h-3.5 w-3.5" />
                              <span>完成 {season.requiredOrders} 委托 / {earlyCost} 金提前解锁</span>
                            </button>
                          )}
                        </div>

                        <div className="mt-2.5 flex flex-wrap gap-3 text-xs text-[#4C554C]">
                          {season.initials.length > 0 && (
                            <span>
                              声母：
                              <strong className="font-display text-[#1F583D]">
                                {season.initials.join(' ')}
                              </strong>
                            </span>
                          )}
                          {season.finals.length > 0 && (
                            <span>
                              韵母：
                              <strong className="font-display text-[#A24B24]">
                                {season.finals.join(' ')}
                              </strong>
                            </span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          )}

          {tab === 'decors' && (
            <div>
              <h3 className="mb-3 text-sm font-bold text-[#253026]">功能性魔法装饰</h3>
              <div className="grid gap-3 sm:grid-cols-2">
                {DECOR_CATALOG.map((decor) => {
                  const owned = state.ownedDecors.includes(decor.id);
                  const affordable = state.coins >= decor.cost;
                  return (
                    <div
                      key={decor.id}
                      className="flex flex-col justify-between rounded-xl border border-[#DDD5C3] bg-[#FFFDF8] p-4"
                    >
                      <div>
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-semibold text-[#222B23]">{decor.name}</span>
                          <span className="font-mono-num text-xs font-semibold text-[#9E6A14]">
                            {owned ? '已拥有' : `${decor.cost} 金`}
                          </span>
                        </div>
                        <p className="mt-1.5 text-xs leading-relaxed text-[#5A6258]">
                          {decor.effect}
                        </p>
                      </div>

                      <button
                        type="button"
                        disabled={owned || !affordable}
                        onClick={() => onBuyDecor(decor.id)}
                        className={`mt-3 inline-flex h-9 w-full items-center justify-center gap-1.5 rounded-lg text-xs font-semibold transition-colors ${
                          owned
                            ? 'bg-[#E6F1E9] text-[#225C40]'
                            : affordable
                              ? 'bg-[#C96F4B] text-[#FFFDF8] hover:bg-[#B45D3B]'
                              : 'cursor-not-allowed bg-[#EAE5D8] text-[#8A8578]'
                        }`}
                      >
                        {owned ? (
                          <>
                            <Check className="h-3.5 w-3.5" />
                            <span>已在温室生效</span>
                          </>
                        ) : (
                          <>
                            <Sparkles className="h-3.5 w-3.5" />
                            <span>购买并布置</span>
                          </>
                        )}
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
