import React from 'react';
import {
  ArrowRight,
  Check,
  Flame,
  FlaskConical,
  Headphones,
  Maximize2,
  Minimize2,
  RotateCcw,
  ShoppingBag,
  Sparkles,
  Sprout,
  Trophy,
  Volume2,
} from 'lucide-react';
import {
  MEDIAL_LETTERS,
  type MedialLetter,
  type ToneNumber,
} from '../data/pinyinResources';
import {
  applyToneMark,
  getExampleChar,
  TONE_META,
  TOTAL_DISCOVERABLE_SYLLABLES,
} from '../game/pinyinEngine';
import {
  ALCHEMY_STAGES,
  getNextChallengeStage,
  getStageConfig,
  type AlchemyStageId,
  type ChallengeStageId,
} from '../game/alchemyStages';
import type { BrewedPotion, LastBrewFeedback, SeedKind } from '../game/simulation';

interface AlembicWorkshopProps {
  unlockedInitials: string[];
  unlockedFinals: string[];
  pantry: Record<string, number>;
  selectedInitial: string;
  selectedMedial: MedialLetter;
  selectedFinal: string;
  selectedTone: ToneNumber;
  hasEmptyPot: boolean;
  activeStageId: AlchemyStageId;
  stageProgress: Record<ChallengeStageId, number>;
  clearedStages: ChallengeStageId[];
  onSelectStage: (stageId: AlchemyStageId) => void;
  onReplayStage: (stageId: ChallengeStageId) => void;
  onSelectInitial: (val: string) => void;
  onSelectMedial: (val: MedialLetter) => void;
  onSelectFinal: (val: string) => void;
  onSelectTone: (tone: ToneNumber) => void;
  onBrew: () => void;
  onClearSelection: () => void;
  onOpenSeedShop: () => void;
  onQuickPlant: (seedId: string, kind: SeedKind) => void;
  onPlayPotion: (syllable: string, tone: ToneNumber, slow?: boolean) => void;
  potions: BrewedPotion[];
  lastBrew: LastBrewFeedback | null;
  /** Focus mode: the host hides the HUD / tab bar, this panel hides its header
   *  and fills the screen so the whole workflow fits without scrolling. */
  focus?: boolean;
  onToggleFocus?: () => void;
}

const ToneContourIcon: React.FC<{ tone: ToneNumber; active: boolean }> = ({ tone, active }) => {
  const stroke = active ? '#FFFDF8' : TONE_META[tone].colorHex;
  const pathByTone: Record<ToneNumber, string> = {
    1: 'M4 9 L20 9',
    2: 'M4 15 L20 6',
    3: 'M4 9 L12 16 L20 7',
    4: 'M4 6 L20 15',
  };
  return (
    <svg viewBox="0 0 24 22" className="h-5 w-6" aria-hidden="true">
      <path
        d={pathByTone[tone]}
        fill="none"
        stroke={stroke}
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
};

export const AlembicWorkshop: React.FC<AlembicWorkshopProps> = ({
  unlockedInitials,
  unlockedFinals,
  pantry,
  selectedInitial,
  selectedMedial,
  selectedFinal,
  selectedTone,
  hasEmptyPot,
  activeStageId,
  stageProgress,
  clearedStages,
  onSelectStage,
  onReplayStage,
  onSelectInitial,
  onSelectMedial,
  onSelectFinal,
  onSelectTone,
  onBrew,
  onClearSelection,
  onOpenSeedShop,
  onQuickPlant,
  onPlayPotion,
  potions,
  lastBrew,
  focus = false,
  onToggleFocus,
}) => {
  const stage = getStageConfig(activeStageId);
  const isChallengeStage = activeStageId !== 'free';
  const currentStep = isChallengeStage ? (stageProgress[activeStageId] ?? 0) : 0;
  const isStageComplete = isChallengeStage && currentStep >= stage.tasks.length;
  const currentTask = isChallengeStage && !isStageComplete ? stage.tasks[currentStep] : null;
  const nextStageId = getNextChallengeStage(activeStageId);

  const visibleInitials = isChallengeStage ? stage.featuredInitials : unlockedInitials;
  const visibleFinals = isChallengeStage ? stage.featuredFinals : unlockedFinals;
  const effectiveMedial: MedialLetter = stage.allowMedial ? selectedMedial : '';

  const hasInitialStock =
    isChallengeStage || selectedInitial === '' || (pantry[selectedInitial] ?? 0) > 0;
  const hasMedialStock =
    isChallengeStage || effectiveMedial === '' || (pantry[effectiveMedial] ?? 0) > 0;
  const hasFinalStock =
    selectedFinal !== '' && (isChallengeStage || (pantry[selectedFinal] ?? 0) > 0);
  const canBrew = hasInitialStock && hasMedialStock && hasFinalStock;

  const missingItem: { id: string; kind: SeedKind } | null = isChallengeStage
    ? null
    : !hasInitialStock
      ? { id: selectedInitial, kind: 'initial' }
      : !hasMedialStock
        ? { id: effectiveMedial, kind: 'final' }
        : selectedFinal !== '' && !hasFinalStock
          ? { id: selectedFinal, kind: 'final' }
          : null;

  const currentTaskChar = currentTask
    ? getExampleChar(currentTask.syllable, currentTask.tone)
    : null;
  const currentTaskMarked = currentTask
    ? applyToneMark(currentTask.syllable, currentTask.tone)
    : '';

  return (
    <section
      aria-label="拼音炼金蒸馏台"
      className={`relative flex h-full flex-col overflow-hidden bg-[#FAF7EE] ${
        focus ? '' : 'rounded-2xl border border-[#CFC6B2] shadow-sm'
      }`}
    >
      {/* Header Bar — hidden in focus mode so the workflow fills the screen */}
      {!focus && (
      <div className="flex shrink-0 items-center justify-between border-b border-[#3E4F40] bg-[#2D3B2F] px-4 py-2.5 text-[#FAF5E8]">
        <div className="min-w-0">
          <h2 className="font-display text-lg font-bold tracking-wide">铜制炼金蒸馏台</h2>
          <p className="truncate text-xs text-[#D5D1C3]">
            {isChallengeStage
              ? `${stage.title} · 试炼免耗素材，通关奖 +${stage.rewardGold} 金币`
              : `自由探索 ${TOTAL_DISCOVERABLE_SYLLABLES} 个两拼与三拼标准音节`}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <button
            type="button"
            onClick={onClearSelection}
            aria-label="清空蒸馏台配方"
            className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-[#566A58] bg-[#222E24] px-2.5 text-xs font-medium text-[#E7E0CE] transition-colors hover:bg-[#344537] whitespace-nowrap shrink-0"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            <span>重置</span>
          </button>
        </div>
      </div>
      )}

      {/* Stage / Difficulty Selector Strip (关卡选项) — hidden in focus mode */}
      {!focus && (
      <div
        role="tablist"
        aria-label="炼金蒸馏台难度关卡选项"
        className="grid shrink-0 grid-cols-5 gap-1 border-b border-[#DFD8C6] bg-[#243126] px-3 py-1.5"
      >
        {ALCHEMY_STAGES.map((st) => {
          const active = activeStageId === st.id;
          const cleared = st.id !== 'free' && clearedStages.includes(st.id);
          return (
            <button
              key={st.id}
              type="button"
              role="tab"
              aria-selected={active}
              data-testid={`stage-tab-${st.id}`}
              onClick={() => onSelectStage(st.id)}
              title={`${st.title}：${st.subtitle}`}
              className={`flex h-8 items-center justify-center gap-1 rounded-lg px-1.5 text-xs font-semibold transition-all whitespace-nowrap ${
                active
                  ? 'bg-[#FAF7EE] text-[#1F4A34] shadow-xs'
                  : 'text-[#D6D2C2] hover:bg-[#314234] hover:text-[#FFFDF8]'
              }`}
            >
              <span>{st.shortLabel}</span>
              {cleared && (
                <Check
                  className={`h-3.5 w-3.5 shrink-0 ${active ? 'text-[#2E7D52]' : 'text-[#8ED0A5]'}`}
                  aria-label="已通关"
                />
              )}
            </button>
          );
        })}
      </div>
      )}

      {/* Active Challenge Stage Mission Strip */}
      {isChallengeStage && (
        <div
          data-testid="stage-mission-bar"
          className={`flex items-center justify-between gap-2 border-b px-4 py-2 ${
            isStageComplete
              ? 'border-[#A7D3B5] bg-[#E8F5EC] text-[#1D4A34]'
              : 'border-[#DFD5BC] bg-[#FBF6E8] text-[#2F3A2F]'
          }`}
        >
          {isStageComplete ? (
            <>
              <div className="flex min-w-0 items-center gap-2 text-xs font-semibold">
                <Trophy className="h-4 w-4 shrink-0 text-[#C6861E]" />
                <span className="truncate">
                  {stage.title} 全部通关（{stage.tasks.length}/{stage.tasks.length}）！
                </span>
              </div>
              <div className="flex shrink-0 items-center gap-1.5">
                <button
                  type="button"
                  data-testid="stage-replay-btn"
                  onClick={() => onReplayStage(activeStageId)}
                  className="inline-flex h-7 items-center gap-1 rounded-lg border border-[#A5C9B0] bg-[#FFFFFF] px-2.5 text-xs font-semibold text-[#1F583D] hover:bg-[#F0FAF3] whitespace-nowrap"
                >
                  <RotateCcw className="h-3 w-3" />
                  <span>重玩</span>
                </button>
                {nextStageId && (
                  <button
                    type="button"
                    data-testid="stage-next-btn"
                    onClick={() => onSelectStage(nextStageId)}
                    className="inline-flex h-7 items-center gap-1 rounded-lg bg-[#2E6F52] px-2.5 text-xs font-semibold text-[#FFFDF8] hover:bg-[#235840] whitespace-nowrap"
                  >
                    <span>下一关</span>
                    <ArrowRight className="h-3 w-3" />
                  </button>
                )}
              </div>
            </>
          ) : currentTask && currentTaskChar ? (
            <>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-1.5 text-xs">
                  <span className="font-mono-num font-bold text-[#9C5D1A]">
                    {'★'.repeat(stage.stars)}
                    {'☆'.repeat(4 - stage.stars)}
                  </span>
                  <span className="font-mono-num font-semibold text-[#4B584E]">
                    第 {currentStep + 1}/{stage.tasks.length} 题
                  </span>
                  <span aria-hidden="true" className="text-[#A8A08E]">·</span>
                  {stage.blindMode ? (
                    <span className="inline-flex items-center gap-1 font-semibold text-[#24667A]">
                      <Headphones className="h-3.5 w-3.5" />
                      <span>盲听辨音（听声配药）</span>
                    </span>
                  ) : (
                    <span className="font-semibold text-[#1F4A34]">
                      目标：<span className="font-display text-sm">{currentTaskMarked}</span>
                      （{currentTaskChar.char} · {currentTaskChar.word}）
                    </span>
                  )}
                </div>
                <p className="mt-0.5 truncate text-[11px] text-[#5D665D]">{currentTask.clue}</p>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <button
                  type="button"
                  data-testid="stage-listen-btn"
                  onClick={() => onPlayPotion(currentTask.syllable, currentTask.tone, false)}
                  className="inline-flex h-7 items-center gap-1 rounded-lg bg-[#2E6F52] px-2.5 text-xs font-semibold text-[#FFFDF8] hover:bg-[#235840] whitespace-nowrap"
                >
                  <Volume2 className="h-3.5 w-3.5" />
                  <span>{stage.blindMode ? '听目标音' : '听音'}</span>
                </button>
                {stage.blindMode && (
                  <button
                    type="button"
                    data-testid="stage-listen-slow-btn"
                    onClick={() => onPlayPotion(currentTask.syllable, currentTask.tone, true)}
                    className="inline-flex h-7 items-center rounded-lg border border-[#B9CBBF] bg-[#FFFDF8] px-2 text-[11px] font-semibold text-[#24667A] hover:bg-[#EAF4F7] whitespace-nowrap"
                  >
                    慢速
                  </button>
                )}
              </div>
            </>
          ) : null}
        </div>
      )}

      {/* Scrollable core: cauldron + ingredients. Keeps the brew CTA and every
          selector reachable when the phone viewport is short. In focus mode the
          content usually fits, so it is vertically centered to split the spare
          room above and below instead of leaving it all at the bottom. */}
      <div
        className={`min-h-0 flex-1 overflow-y-auto ${
          focus ? 'flex flex-col justify-center' : ''
        }`}
      >
      {/* Centerpiece: cauldron on the left, phonetic equation on the right —
          one row on phones too, so the whole workflow fits a single screen */}
      <div className="border-b border-[#DFD8C6] bg-[#F2EDE0] px-2.5 py-2 sm:px-4 sm:py-2.5">
        <div className="flex flex-row items-center gap-2 sm:gap-2.5">
          {/* Illustrated Copper Alembic SVG — tapping the cauldron toggles
              focus mode (hide header/HUD/tabs, fill the screen) */}
          <button
            type="button"
            onClick={onToggleFocus}
            aria-label={focus ? '还原炼金台布局' : '最大化炼金台'}
            aria-pressed={focus}
            data-testid="cauldron-focus-toggle"
            title={focus ? '还原' : '最大化：隐藏顶部，操作区占满全屏'}
            className="relative flex shrink-0 items-center justify-center rounded-xl transition-transform active:scale-95 focus-visible:outline-2 focus-visible:outline-[#2E6F52]"
          >
            <svg viewBox="0 0 132 116" className="h-16 w-16 sm:h-20 sm:w-24" aria-hidden="true">
              {/* Flame underneath */}
              <path
                d="M52 106 Q62 86 72 106 Q62 111 52 106 Z"
                fill={TONE_META[selectedTone].colorHex}
                opacity="0.88"
              />
              <path d="M57 105 Q62 94 67 105 Z" fill="#FDE6A6" />

              {/* Iron tripod */}
              <path
                d="M36 108 L46 86 M88 108 L78 86 M40 94 L84 94"
                stroke="#3C3B37"
                strokeWidth="3"
                strokeLinecap="round"
              />

              {/* Copper boiler sphere */}
              <circle
                cx="62"
                cy="68"
                r="26"
                fill="#C77647"
                stroke="#7A4021"
                strokeWidth="2"
              />
              <path
                d="M44 58 A20 20 0 0 1 64 46"
                fill="none"
                stroke="#F0B98C"
                strokeWidth="3"
                strokeLinecap="round"
                opacity="0.7"
              />

              {/* Glass viewing porthole with liquid */}
              <circle cx="62" cy="68" r="14" fill="#233126" stroke="#D9B166" strokeWidth="2.5" />
              <path
                d="M50 70 Q56 66 62 70 Q68 74 74 70 A12 12 0 1 1 50 70 Z"
                fill={TONE_META[selectedTone].colorHex}
                opacity="0.9"
              />

              {/* Animated bubbles */}
              <circle cx="58" cy="66" r="2.2" fill="#FFFDF8" className="animate-bubble-1" />
              <circle cx="65" cy="69" r="1.8" fill="#FFFDF8" className="animate-bubble-2" />
              <circle cx="62" cy="64" r="1.5" fill="#FDE6A6" className="animate-bubble-3" />

              {/* Alembic dome & condenser tube */}
              <path
                d="M50 44 L54 24 Q62 18 70 24 L74 44 Z"
                fill="#D48957"
                stroke="#7A4021"
                strokeWidth="1.8"
              />
              <path
                d="M66 24 C84 12 104 20 104 42 L104 68"
                fill="none"
                stroke="#B86B3D"
                strokeWidth="5"
                strokeLinecap="round"
              />
              <path
                d="M66 24 C84 12 104 20 104 42 L104 68"
                fill="none"
                stroke="#EAB68A"
                strokeWidth="1.5"
                strokeLinecap="round"
                opacity="0.7"
              />

              {/* Receiving flask */}
              <path
                d="M95 72 L113 72 L119 98 Q104 104 89 98 Z"
                fill="#DCEAE7"
                stroke="#4B6660"
                strokeWidth="1.8"
                opacity="0.92"
              />
              <path
                d="M93 85 L115 85 L118 97 Q104 102 90 97 Z"
                fill={TONE_META[selectedTone].colorHex}
                opacity="0.78"
              />
            </svg>
            {/* Corner hint: tap the cauldron to expand / restore */}
            <span className="pointer-events-none absolute -right-0.5 -top-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-[#2D3B2F]/85 text-[#F5EFE0]">
              {focus ? <Minimize2 className="h-2.5 w-2.5" /> : <Maximize2 className="h-2.5 w-2.5" />}
            </span>
          </button>

          {/* Live Phonetic Equation & Brew CTA */}
          <div className="flex min-w-0 flex-1 flex-col items-start">
            <div className="flex flex-wrap items-center justify-start gap-1 sm:gap-1.5">
              <div className="flex min-w-[38px] flex-col items-center rounded-lg border border-[#C2D4C6] bg-[#FFFDF8] px-1.5 py-0.5 sm:min-w-[50px] sm:rounded-xl sm:px-2">
                <span className="text-[9px] text-[#5B685E] sm:text-[10px]">声母</span>
                <span
                  data-testid="selected-initial"
                  className="font-display text-base font-bold text-[#1F583D]"
                >
                  {selectedInitial === '' ? '零' : selectedInitial}
                </span>
              </div>

              {stage.allowMedial && (
                <>
                  <span className="font-display text-sm font-bold text-[#7C7565]">+</span>
                  <div
                    className={`flex min-w-[38px] flex-col items-center rounded-lg border px-1.5 py-0.5 sm:min-w-[50px] sm:rounded-xl sm:px-2 ${
                      effectiveMedial !== ''
                        ? 'border-[#77A5B5] bg-[#EEF6F8]'
                        : 'border-dashed border-[#CFC8B6] bg-[#F8F5EC]'
                    }`}
                  >
                    <span className="text-[9px] text-[#4C6A73] sm:text-[10px]">介母</span>
                    <span
                      data-testid="selected-medial"
                      className={`font-display text-base font-bold ${
                        effectiveMedial !== '' ? 'text-[#24667A]' : 'text-[#9B968A]'
                      }`}
                    >
                      {effectiveMedial === '' ? '无' : effectiveMedial}
                    </span>
                  </div>
                </>
              )}

              <span className="font-display text-sm font-bold text-[#7C7565]">+</span>

              <div className="flex min-w-[38px] flex-col items-center rounded-lg border border-[#E1C6B5] bg-[#FFFDF8] px-1.5 py-0.5 sm:min-w-[50px] sm:rounded-xl sm:px-2">
                <span className="text-[9px] text-[#7D5B47] sm:text-[10px]">韵母</span>
                <span
                  data-testid="selected-final"
                  className="font-display text-base font-bold text-[#A24B24]"
                >
                  {selectedFinal || '？'}
                </span>
              </div>

              <span className="font-display text-sm font-bold text-[#7C7565]">·</span>

              <div
                className="flex min-w-[48px] flex-col items-center rounded-lg border px-1.5 py-0.5 sm:min-w-[64px] sm:rounded-xl sm:px-2"
                style={{
                  borderColor: `${TONE_META[selectedTone].colorHex}55`,
                  backgroundColor: '#FFFDF8',
                }}
              >
                <span className="text-[9px] text-[#5F655E] sm:text-[10px]">声调</span>
                <span
                  className="text-xs font-bold leading-6"
                  style={{ color: TONE_META[selectedTone].colorHex }}
                >
                  {TONE_META[selectedTone].short}
                </span>
              </div>
            </div>

            <div className="mt-2 flex flex-wrap items-center gap-2">
              <button
                type="button"
                data-testid="brew-btn"
                disabled={!canBrew}
                onClick={onBrew}
                className={`inline-flex h-9 items-center gap-2 rounded-xl px-4 text-sm font-semibold shadow-xs transition-all duration-150 whitespace-nowrap shrink-0 ${
                  canBrew
                    ? 'bg-[#2E6F52] text-[#FFFDF8] hover:bg-[#235840] active:scale-[0.98]'
                    : 'cursor-not-allowed bg-[#CFC9BA] text-[#6E6B62]'
                }`}
              >
                <FlaskConical className="h-4 w-4" />
                <span>
                  {isChallengeStage
                    ? '提交试炼炼制'
                    : effectiveMedial
                      ? '炼制三拼魔药'
                      : '开始炼制魔药'}
                </span>
              </button>

              {!canBrew && selectedFinal === '' && (
                <span className="text-xs text-[#8C5B32]">请在下方选择韵母素材</span>
              )}

              {missingItem && (
                <div
                  data-testid="out-of-stock-help"
                  className="flex flex-wrap items-center gap-1.5"
                >
                  <button
                    type="button"
                    data-testid="open-seed-shop-btn"
                    onClick={onOpenSeedShop}
                    className="inline-flex h-8 items-center gap-1 rounded-lg bg-[#C96F4B] px-2.5 text-xs font-semibold text-[#FFFDF8] hover:bg-[#B45D3B] whitespace-nowrap shrink-0"
                  >
                    <ShoppingBag className="h-3.5 w-3.5" />
                    <span>商店买「{missingItem.id}」</span>
                  </button>
                  {hasEmptyPot && (
                    <button
                      type="button"
                      data-testid="quick-plant-missing-btn"
                      onClick={() => onQuickPlant(missingItem.id, missingItem.kind)}
                      className="inline-flex h-8 items-center gap-1 rounded-lg border border-[#B9CEBF] bg-[#FFFDF8] px-2.5 text-xs font-semibold text-[#1F583D] hover:bg-[#EAF4ED] whitespace-nowrap shrink-0"
                    >
                      <Sprout className="h-3.5 w-3.5" />
                      <span>种入空盆</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Last Brew Feedback Callout */}
        {lastBrew && (
          <div
            key={lastBrew.id}
            data-testid="brew-feedback"
            className={`animate-brew-pop mt-2 flex items-center justify-between gap-2.5 rounded-xl border px-3 py-1.5 ${
              lastBrew.success
                ? 'border-[#9AC9A9] bg-[#EDF7F0] text-[#1D4A34]'
                : 'border-[#D9B6B2] bg-[#FBF0EE] text-[#7D2F29]'
            }`}
          >
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-1.5 text-xs font-semibold">
                {lastBrew.success ? (
                  <>
                    <Sparkles className="h-3.5 w-3.5 shrink-0 text-[#2E7D52]" />
                    <span className="font-display text-sm">{lastBrew.marked}</span>
                    <span>
                      「{lastBrew.char} · {lastBrew.word}」
                    </span>
                    <span className="font-normal text-[#35634B]">— {lastBrew.message}</span>
                  </>
                ) : (
                  <span>{lastBrew.message}</span>
                )}
              </div>
              <p className="mt-0.5 truncate text-[11px] opacity-90">{lastBrew.ruleNote}</p>
            </div>
            {lastBrew.success && (
              <button
                type="button"
                onClick={() => onPlayPotion(lastBrew.syllable, lastBrew.tone)}
                aria-label={`朗读刚炼成的 ${lastBrew.marked}`}
                className="inline-flex h-7 shrink-0 items-center gap-1 rounded-lg border border-[#9AC9A9] bg-[#FFFFFF] px-2 text-xs font-semibold text-[#1D4A34] hover:bg-[#E1F2E6]"
              >
                <Volume2 className="h-3 w-3" />
                <span>再听</span>
              </button>
            )}
          </div>
        )}
      </div>

      {/* Ingredients & Tone Controls */}
      <div className="space-y-2.5 p-3.5">
        {/* Step 1: Initials */}
        <div>
          <div className="mb-1 flex items-center justify-between text-xs">
            <span className="font-semibold text-[#23543C]">
              1. 选择声母露珠{isChallengeStage ? '（本关精选）' : '（可不选作零声母）'}
            </span>
            <span className="text-[#6B7166]">{isChallengeStage ? '试炼免耗库存' : '右下角为库存'}</span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {!isChallengeStage && (
              <button
                type="button"
                onClick={() => onSelectInitial('')}
                className={`h-8 rounded-lg border px-2.5 text-xs font-semibold transition-colors whitespace-nowrap ${
                  selectedInitial === ''
                    ? 'border-[#2E6F52] bg-[#2E6F52] text-[#FFFDF8]'
                    : 'border-[#D4CEBE] bg-[#FFFDF8] text-[#4E574E] hover:border-[#94B5A0]'
                }`}
              >
                零声母
              </button>
            )}
            {visibleInitials.map((ini) => {
              const count = pantry[ini] ?? 0;
              const active = selectedInitial === ini;
              const available = isChallengeStage || count > 0;
              return (
                <button
                  key={ini}
                  type="button"
                  data-testid={`initial-${ini}`}
                  onClick={() => onSelectInitial(ini)}
                  className={`relative flex h-8 min-w-[40px] items-center justify-center rounded-lg border px-2 font-display text-sm font-bold transition-colors ${
                    active
                      ? 'border-[#235840] bg-[#2E6F52] text-[#FFFDF8] shadow-xs'
                      : available
                        ? 'border-[#C6D5C9] bg-[#FFFDF8] text-[#1F583D] hover:bg-[#EAF4ED]'
                        : 'border-[#E2DDD0] bg-[#F3EFE5] text-[#9A968B]'
                  }`}
                >
                  <span>{ini}</span>
                  {!isChallengeStage && (
                    <span
                      className={`font-mono-num ml-1 text-[10px] font-medium ${
                        active ? 'text-[#D3F2DF]' : count > 0 ? 'text-[#4D7861]' : 'text-[#A8A396]'
                      }`}
                    >
                      {count}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Step 2: Optional Medial for Three-Part Syllables (hidden in Stage 1 Beginner Mode) */}
        {stage.allowMedial && (
          <div data-testid="medial-selector-row">
            <div className="mb-1 flex items-center justify-between text-xs">
              <span className="font-semibold text-[#24667A]">
                2. 介母槽（合成 jia / duo / hua / juan 等三拼音节）
              </span>
              <span className="text-[#6B7166]">选「无」为两拼</span>
            </div>
            <div className="flex flex-wrap gap-1.5">
              <button
                type="button"
                data-testid="medial-none"
                onClick={() => onSelectMedial('')}
                className={`h-8 rounded-lg border px-2.5 text-xs font-semibold transition-colors whitespace-nowrap ${
                  selectedMedial === ''
                    ? 'border-[#24667A] bg-[#24667A] text-[#FFFDF8]'
                    : 'border-[#D4CEBE] bg-[#FFFDF8] text-[#4E574E] hover:border-[#8DB4C2]'
                }`}
              >
                无（两拼）
              </button>
              {MEDIAL_LETTERS.map((med) => {
                const count = pantry[med] ?? 0;
                const active = selectedMedial === med;
                const available = isChallengeStage || count > 0;
                return (
                  <button
                    key={med}
                    type="button"
                    data-testid={`medial-${med}`}
                    onClick={() => onSelectMedial(med)}
                    className={`flex h-8 min-w-[48px] items-center justify-center rounded-lg border px-2.5 font-display text-sm font-bold transition-colors ${
                      active
                        ? 'border-[#1E5566] bg-[#24667A] text-[#FFFDF8] shadow-xs'
                        : available
                          ? 'border-[#B9D4DC] bg-[#FFFDF8] text-[#24667A] hover:bg-[#EAF4F7]'
                          : 'border-[#E2DDD0] bg-[#F3EFE5] text-[#9A968B]'
                    }`}
                  >
                    <span>{med}</span>
                    {!isChallengeStage && (
                      <span
                        className={`font-mono-num ml-1 text-[10px] font-medium ${
                          active ? 'text-[#CDEAF2]' : count > 0 ? 'text-[#4B7E8C]' : 'text-[#A8A396]'
                        }`}
                      >
                        {count}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Step 3 (or 2 in Stage 1): Finals */}
        <div>
          <div className="mb-1 flex items-center justify-between text-xs">
            <span className="font-semibold text-[#9C4822]">
              {stage.allowMedial ? '3.' : '2.'} 选择韵母花粉（必填基底）
            </span>
            <span className="text-[#6B7166]">ü 遇 j/q/x/y 自动去点</span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {visibleFinals.map((fin) => {
              const count = pantry[fin] ?? 0;
              const active = selectedFinal === fin;
              const available = isChallengeStage || count > 0;
              return (
                <button
                  key={fin}
                  type="button"
                  data-testid={`final-${fin}`}
                  onClick={() => onSelectFinal(fin)}
                  className={`relative flex h-8 min-w-[42px] items-center justify-center rounded-lg border px-2 font-display text-sm font-bold transition-colors ${
                    active
                      ? 'border-[#9C4822] bg-[#C86A3B] text-[#FFFDF8] shadow-xs'
                      : available
                        ? 'border-[#E2C9B9] bg-[#FFFDF8] text-[#9C4822] hover:bg-[#F9EFE8]'
                        : 'border-[#E2DDD0] bg-[#F3EFE5] text-[#9A968B]'
                  }`}
                >
                  <span>{fin}</span>
                  {!isChallengeStage && (
                    <span
                      className={`font-mono-num ml-1 text-[10px] font-medium ${
                        active ? 'text-[#FFE7D6]' : count > 0 ? 'text-[#A86A4D]' : 'text-[#A8A396]'
                      }`}
                    >
                      {count}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Step 4 (or 3 in Stage 1): 4-Tone Flame Selector */}
        <div>
          <div className="mb-1 flex items-center gap-1.5 text-xs font-semibold text-[#4B463B]">
            <Flame className="h-3.5 w-3.5 text-[#C86A3B]" />
            <span>{stage.allowMedial ? '4.' : '3.'} 调节声调火候</span>
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {([1, 2, 3, 4] as ToneNumber[]).map((tone) => {
              const active = selectedTone === tone;
              const meta = TONE_META[tone];
              return (
                <button
                  key={tone}
                  type="button"
                  data-testid={`tone-${tone}`}
                  onClick={() => onSelectTone(tone)}
                  style={
                    active
                      ? { backgroundColor: meta.colorHex, borderColor: meta.colorHex }
                      : undefined
                  }
                  className={`flex h-9 items-center justify-center gap-1.5 rounded-xl border px-2 text-xs font-semibold transition-all whitespace-nowrap ${
                    active
                      ? 'text-[#FFFDF8] shadow-xs'
                      : 'border-[#D8D1C0] bg-[#FFFDF8] text-[#3A4038] hover:border-[#ACA491]'
                  }`}
                >
                  <ToneContourIcon tone={tone} active={active} />
                  <span>{meta.short}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
      </div>

      {/* Brewed Potion Shelf */}
      <div className="mt-auto shrink-0 border-t border-[#DFD8C6] bg-[#F3EFE3] px-4 py-2.5">
        <div className="mb-1 flex items-center justify-between text-xs">
          <span className="font-semibold text-[#3C463C]">
            成品药水架（点击药水瓶听发音）
          </span>
          <span className="font-mono-num text-[#686D63]">
            共 {potions.reduce((s, p) => s + p.count, 0)} 瓶
          </span>
        </div>

        {potions.length === 0 ? (
          <p className="py-0.5 text-xs text-[#73786E]">
            {isChallengeStage
              ? '按上方关卡目标调配声母、韵母与声调，炼出魔药即可通关赢取金币！'
              : '试试选「j」+ 介母「i」+「a」+「一声平」，或切换上方 ①~④ 关卡挑战！'}
          </p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {potions.slice(0, 6).map((potion) => (
              <button
                key={potion.id}
                type="button"
                data-testid={`potion-${potion.syllable}-${potion.tone}`}
                onClick={() => onPlayPotion(potion.syllable, potion.tone)}
                className="group flex items-center gap-2 rounded-xl border border-[#D4CCB9] bg-[#FFFDF8] px-2.5 py-1 text-left transition-transform duration-150 hover:-translate-y-0.5 hover:border-[#2E6F52]"
              >
                <span
                  className="flex h-6 w-6 shrink-0 items-center justify-center rounded-lg text-xs font-bold text-[#FFFDF8]"
                  style={{ backgroundColor: potion.colorHex }}
                >
                  {potion.char}
                </span>
                <div>
                  <div className="flex items-baseline gap-1">
                    <span className="font-display text-sm font-bold leading-tight text-[#232B23]">
                      {potion.marked}
                    </span>
                    {potion.isTriplet && (
                      <span className="text-[10px] font-semibold text-[#24667A]">三拼</span>
                    )}
                  </div>
                  <div className="text-[10px] text-[#666C62]">
                    {potion.word} · ×{potion.count}
                  </div>
                </div>
                <Volume2 className="ml-0.5 h-3 w-3 text-[#7B8278] group-hover:text-[#2E6F52]" />
              </button>
            ))}
          </div>
        )}
      </div>
    </section>
  );
};
