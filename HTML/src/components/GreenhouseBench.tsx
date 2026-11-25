import React, { useState } from 'react';
import { Coins, Droplets, Plus, Sparkles, Trash2, Volume2 } from 'lucide-react';
import {
  GREENHOUSE_IMAGES,
  POT_THEMES,
  type PotThemeId,
} from '../data/pinyinResources';
import { getSeedMnemonic } from '../game/pinyinEngine';
import {
  MAX_PLOTS,
  getNextPlotCost,
  type PlotState,
  type SeedKind,
} from '../game/simulation';

interface GreenhouseBenchProps {
  plots: PlotState[];
  potTheme: PotThemeId;
  unlockedInitials: string[];
  unlockedFinals: string[];
  pantry: Record<string, number>;
  energy: number;
  coins: number;
  hasAutoSprinkler: boolean;
  onWaterPlot: (plotId: number) => void;
  onHarvestPlot: (plotId: number) => void;
  onPlantSeed: (plotId: number, seedId: string, kind: SeedKind) => void;
  onClearPlot: (plotId: number) => void;
  onExpandPlot: () => void;
  onSpeakSeed: (seedId: string) => void;
}

const PlantGraphic: React.FC<{
  plot: PlotState;
  potColors: { bodyHex: string; rimHex: string; shadowHex: string };
}> = ({ plot, potColors }) => {
  const isInitial = plot.kind === 'initial';
  const leafMain = isInitial ? '#38875E' : '#C86D3D';
  const leafLight = isInitial ? '#74C494' : '#EE9E66';
  const bloomColor = isInitial ? '#D6F3DF' : '#FFE39E';

  return (
    <svg viewBox="0 0 120 112" className="mx-auto h-16 w-20 sm:h-24 sm:w-28" aria-hidden="true">
      {/* Ground shadow */}
      <ellipse cx="60" cy="103" rx="34" ry="5" fill="#1F281F" opacity="0.16" />

      {/* Plant foliage based on growth stage */}
      {plot.seedId && (
        <g className={plot.growth === 2 ? 'animate-float-slow' : ''}>
          {/* Stem */}
          <path
            d={
              plot.growth === 0
                ? 'M60 66 Q60 56 60 51'
                : plot.growth === 1
                  ? 'M60 66 Q58 48 60 36'
                  : 'M60 66 Q56 44 60 24'
            }
            fill="none"
            stroke="#3A6E47"
            strokeWidth="3.5"
            strokeLinecap="round"
          />

          {/* Stage 0: Sprout cotyledons */}
          {plot.growth >= 0 && (
            <>
              <path
                d="M60 54 C51 48 44 50 47 56 C50 61 57 58 60 54 Z"
                fill={leafMain}
              />
              <path
                d="M60 54 C69 48 76 50 73 56 C70 61 63 58 60 54 Z"
                fill={leafLight}
              />
            </>
          )}

          {/* Stage 1+: Secondary lush leaves */}
          {plot.growth >= 1 && (
            <>
              <path
                d="M60 41 C46 34 38 39 42 46 C46 52 56 47 60 41 Z"
                fill={leafMain}
              />
              <path
                d="M60 40 C74 33 82 38 78 45 C74 51 64 46 60 40 Z"
                fill={leafLight}
              />
            </>
          )}

          {/* Stage 2: Radiant harvest-ready botanical crystal / blossom */}
          {plot.growth === 2 && (
            <g>
              <circle cx="60" cy="22" r="14" fill={leafMain} opacity="0.22" />
              {isInitial ? (
                <polygon
                  points="60,8 71,21 60,34 49,21"
                  fill={leafLight}
                  stroke="#204D36"
                  strokeWidth="1.6"
                />
              ) : (
                <g>
                  <circle cx="60" cy="14" r="6" fill={leafLight} />
                  <circle cx="68" cy="21" r="6" fill={leafMain} />
                  <circle cx="60" cy="28" r="6" fill={leafLight} />
                  <circle cx="52" cy="21" r="6" fill={leafMain} />
                  <circle cx="60" cy="21" r="5" fill={bloomColor} />
                </g>
              )}
              <circle cx="40" cy="18" r="2.2" fill="#F4CF7B" />
              <circle cx="81" cy="15" r="1.8" fill="#F4CF7B" />
            </g>
          )}
        </g>
      )}

      {/* Soil mound */}
      <ellipse cx="60" cy="68" rx="28" ry="6" fill="#4A3526" />

      {/* Terracotta / themed pot body */}
      <path
        d="M32 68 L39 101 Q60 106 81 101 L88 68 Z"
        fill={potColors.bodyHex}
        stroke={potColors.shadowHex}
        strokeWidth="1.6"
      />
      {/* Pot shading */}
      <path
        d="M72 68 L69 102 Q76 102 81 101 L88 68 Z"
        fill={potColors.shadowHex}
        opacity="0.28"
      />
      {/* Pot rim */}
      <rect
        x="27"
        y="61"
        width="66"
        height="10"
        rx="4"
        fill={potColors.rimHex}
        stroke={potColors.shadowHex}
        strokeWidth="1.6"
      />

      {/* Phonetic letter engraved on pot medallion */}
      {plot.seedId ? (
        <g>
          <circle cx="60" cy="86" r="12.5" fill="#FAF6EB" stroke={potColors.shadowHex} strokeWidth="1.4" />
          <text
            x="60"
            y="91"
            textAnchor="middle"
            fill={isInitial ? '#1F583D' : '#A24B24'}
            style={{
              fontFamily: "'Fraunces', serif",
              fontWeight: 700,
              fontSize: plot.seedId.length > 2 ? '11px' : '15px',
            }}
          >
            {plot.seedId}
          </text>
        </g>
      ) : (
        <text
          x="60"
          y="89"
          textAnchor="middle"
          fill="#FAF6EB"
          opacity="0.85"
          style={{ fontSize: '11px', fontWeight: 600 }}
        >
          空盆
        </text>
      )}
    </svg>
  );
};

export const GreenhouseBench: React.FC<GreenhouseBenchProps> = ({
  plots,
  potTheme,
  unlockedInitials,
  unlockedFinals,
  pantry,
  energy,
  coins,
  hasAutoSprinkler,
  onWaterPlot,
  onHarvestPlot,
  onPlantSeed,
  onClearPlot,
  onExpandPlot,
  onSpeakSeed,
}) => {
  const [selectedPlotId, setSelectedPlotId] = useState<number>(0);
  const [seedTab, setSeedTab] = useState<SeedKind>('initial');

  const theme = POT_THEMES.find((t) => t.id === potTheme) ?? POT_THEMES[0];
  const selectedPlot = plots.find((p) => p.id === selectedPlotId) ?? plots[0];
  const nextPlotCost = getNextPlotCost(plots.length);
  const canAffordExpand = nextPlotCost !== null && coins >= nextPlotCost;

  const handlePlotPrimaryAction = (plot: PlotState) => {
    setSelectedPlotId(plot.id);
    if (!plot.seedId) return;
    onSpeakSeed(plot.seedId);
    if (plot.growth === 2) {
      onHarvestPlot(plot.id);
    } else {
      onWaterPlot(plot.id);
    }
  };

  const handleExpandClick = () => {
    if (!canAffordExpand) return;
    const newPlotId = plots.length;
    onExpandPlot();
    setSelectedPlotId(newPlotId);
  };

  return (
    <section
      aria-label="魔法拼音温室花架"
      className="flex h-full flex-col overflow-hidden rounded-2xl border border-[#CFC6B2] bg-[#FAF7EE] shadow-sm"
    >
      {/* Sunlit Conservatory Header Window — shorter on phones to give the
          pots more room */}
      <div className="relative h-[72px] shrink-0 overflow-hidden border-b border-[#D8D0BE] sm:h-28">
        <img
          src={GREENHOUSE_IMAGES.backdrop}
          alt="温室拱窗与晨光"
          referrerPolicy="no-referrer"
          className="h-full w-full object-cover object-center"
        />
        <div
          className="absolute inset-0"
          style={{
            background:
              'linear-gradient(180deg, rgba(24,35,25,0.22) 0%, rgba(24,35,25,0.78) 100%)',
          }}
        />
        <div className="absolute inset-x-0 bottom-0 flex items-end justify-between gap-2 px-3 py-2 text-[#FAF5E8] sm:px-4 sm:py-3">
          <div className="min-w-0">
            <div className="flex items-baseline gap-2">
              <h2 className="font-display text-base font-bold tracking-wide whitespace-nowrap sm:text-lg">
                温室栽培花架
              </h2>
              <span
                data-testid="plot-capacity"
                className="font-mono-num text-xs font-medium text-[#E5C886]"
              >
                {plots.length}/{MAX_PLOTS} 盆
              </span>
            </div>
            <p className="hidden truncate text-xs text-[#E7E0CE] sm:block">
              {hasAutoSprinkler
                ? '自动洒水器运行中 · 点击幼苗浇水或收获'
                : '点击幼苗浇水并跟读发音 · 成熟后点击收获'}
            </p>
          </div>
          {nextPlotCost !== null && (
            <button
              type="button"
              data-testid="expand-plot-btn"
              disabled={!canAffordExpand}
              onClick={handleExpandClick}
              title={
                canAffordExpand
                  ? `花费 ${nextPlotCost} 金币增加第 ${plots.length + 1} 号花盆`
                  : `需要 ${nextPlotCost} 金币扩建花架`
              }
              className={`inline-flex h-8 shrink-0 items-center gap-1 rounded-lg border px-2.5 text-xs font-semibold transition-colors whitespace-nowrap ${
                canAffordExpand
                  ? 'border-[#F2D399]/70 bg-[#C96F4B] text-[#FFFDF8] hover:bg-[#B85E3A]'
                  : 'cursor-not-allowed border-[#7C887D]/50 bg-[#263228]/80 text-[#C8C3B4]'
              }`}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>扩建</span>
              <span className="font-mono-num inline-flex items-center gap-0.5 text-[11px]">
                <Coins className="h-3 w-3" />
                {nextPlotCost}
              </span>
            </button>
          )}
        </div>
      </div>

      {/* Botanical Pots Grid — three small pots per row on phones so a screen
          shows more at once; scrolls when the greenhouse expands */}
      <div className="grid min-h-0 flex-1 grid-cols-3 content-start gap-2 overflow-y-auto p-2.5 sm:gap-3 sm:p-4">
        {plots.map((plot) => {
          const isSelected = plot.id === selectedPlotId;
          const isReady = plot.seedId !== null && plot.growth === 2;
          const stock = plot.seedId ? (pantry[plot.seedId] ?? 0) : 0;

          return (
            <div
              key={plot.id}
              className={`relative flex flex-col justify-between rounded-xl border p-1.5 transition-all duration-150 sm:p-2.5 ${
                isSelected
                  ? 'border-[#2E6F52] bg-[#F1F6F0] shadow-sm'
                  : 'border-[#DDD5C3] bg-[#FFFDF8] hover:border-[#B6AC96]'
              }`}
            >
              {/* Top metadata line (unboxed typography) */}
              <div className="flex items-center justify-between text-[10px] text-[#5B6358] sm:text-[11px]">
                <span>
                  {plot.seedId
                    ? plot.kind === 'initial'
                      ? `声母 · 存${stock}`
                      : `韵母 · 存${stock}`
                    : `${plot.id + 1}号空盆`}
                </span>
                {plot.seedId && (
                  <button
                    type="button"
                    onClick={() => onClearPlot(plot.id)}
                    aria-label={`清空 ${plot.id + 1} 号花盆 ${plot.seedId}`}
                    title="铲除改种其他拼音"
                    className="rounded p-1 text-[#7B8077] hover:bg-[#EAE3D2] hover:text-[#9A3E36]"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                )}
              </div>

              {/* Pot clickable area */}
              <button
                type="button"
                data-testid={`plot-${plot.id}`}
                onClick={() => handlePlotPrimaryAction(plot)}
                aria-label={
                  plot.seedId
                    ? isReady
                      ? `收获 ${plot.seedId}`
                      : `给 ${plot.seedId} 浇水并听读音`
                    : `选中 ${plot.id + 1} 号空花盆播种`
                }
                className="group my-1 flex flex-col items-center focus-visible:outline-2 focus-visible:outline-[#2E6F52]"
              >
                <PlantGraphic plot={plot} potColors={theme} />

                {/* Growth stage indicator + explicit action label */}
                <div className="mt-0.5 flex items-center gap-1 text-[10px] font-semibold sm:mt-1 sm:gap-1.5 sm:text-xs">
                  {!plot.seedId ? (
                    <span className="inline-flex items-center gap-1 text-[#6C6A5F]">
                      <Plus className="h-3.5 w-3.5" />
                      选中播种
                    </span>
                  ) : isReady ? (
                    <span className="inline-flex items-center gap-1 text-[#246848]">
                      <Sparkles className="h-3.5 w-3.5" />
                      可收获 (+3)
                    </span>
                  ) : (
                    <span
                      className={`inline-flex items-center gap-1 ${
                        energy > 0 ? 'text-[#2B6E82]' : 'text-[#8C5B32]'
                      }`}
                    >
                      <Droplets className="h-3.5 w-3.5" />
                      {plot.growth === 0 ? '浇水发芽 (1/2)' : '浇水开花 (2/2)'}
                    </span>
                  )}
                </div>
              </button>
            </div>
          );
        })}
      </div>

      {/* Selected Plant Mnemonic Placard OR Seed Planter */}
      <div className="mt-auto shrink-0 border-t border-[#DFD8C6] bg-[#F3EFE3] p-2.5 sm:p-4">
        {selectedPlot.seedId && selectedPlot.kind ? (
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center gap-2 text-xs text-[#5E675C]">
                <span>{selectedPlot.kind === 'initial' ? '声母晶芽口诀' : '韵母灵花口诀'}</span>
                <span aria-hidden="true">·</span>
                <span className="font-display text-sm font-bold text-[#224C37]">
                  {selectedPlot.seedId}
                </span>
              </div>
              <p className="mt-1 text-sm font-medium leading-snug text-[#273128]">
                {getSeedMnemonic(selectedPlot.seedId, selectedPlot.kind)}
              </p>
            </div>
            <button
              type="button"
              onClick={() => onSpeakSeed(selectedPlot.seedId!)}
              aria-label={`朗读 ${selectedPlot.seedId}`}
              className="inline-flex h-10 shrink-0 items-center gap-1.5 rounded-lg border border-[#BFCAB9] bg-[#FFFFFF] px-3 text-xs font-semibold text-[#23543C] transition-colors hover:bg-[#EAF3EC]"
            >
              <Volume2 className="h-4 w-4" />
              <span>听发音</span>
            </button>
          </div>
        ) : (
          <div>
            <div className="mb-2.5 flex items-center justify-between gap-2">
              <span className="text-xs font-semibold text-[#3C463C]">
                为 {selectedPlot.id + 1} 号空盆选择种子
              </span>
              <div className="flex items-center gap-1 rounded-lg bg-[#E4DEC9] p-0.5">
                <button
                  type="button"
                  onClick={() => setSeedTab('initial')}
                  className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors whitespace-nowrap ${
                    seedTab === 'initial'
                      ? 'bg-[#FFFDF8] text-[#1E5239] shadow-xs'
                      : 'text-[#5A6258] hover:text-[#232B23]'
                  }`}
                >
                  声母 ({unlockedInitials.length})
                </button>
                <button
                  type="button"
                  onClick={() => setSeedTab('final')}
                  className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors whitespace-nowrap ${
                    seedTab === 'final'
                      ? 'bg-[#FFFDF8] text-[#A24B24] shadow-xs'
                      : 'text-[#5A6258] hover:text-[#232B23]'
                  }`}
                >
                  韵母 ({unlockedFinals.length})
                </button>
              </div>
            </div>

            <div className="flex flex-wrap gap-1.5">
              {(seedTab === 'initial' ? unlockedInitials : unlockedFinals).map((seed) => (
                <button
                  key={seed}
                  type="button"
                  onClick={() => {
                    onPlantSeed(selectedPlot.id, seed, seedTab);
                    onSpeakSeed(seed);
                  }}
                  className={`h-9 min-w-[40px] rounded-lg border px-2.5 font-display text-sm font-bold transition-colors ${
                    seedTab === 'initial'
                      ? 'border-[#C2D4C6] bg-[#FFFDF8] text-[#1F583D] hover:bg-[#E7F2EA]'
                      : 'border-[#E1C6B5] bg-[#FFFDF8] text-[#A24B24] hover:bg-[#F9EDE6]'
                  }`}
                >
                  {seed}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </section>
  );
};
