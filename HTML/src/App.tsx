import { useCallback, useEffect, useState } from 'react';
import type {
  DecorId,
  MedialLetter,
  PotThemeId,
  ToneNumber,
} from './data/pinyinResources';
import { soundEngine } from './game/audioPlayer';
import {
  advanceDay,
  brewPotion,
  buyDecor,
  buyOrSelectPotTheme,
  buySeedPack,
  clearPlot,
  createInitialGameState,
  expandGreenhousePlot,
  fulfillOrder,
  harvestPlot,
  plantSeed,
  quickPlantToEmptyPlot,
  replayAlchemyStage,
  revealOrderHint,
  selectAlchemyStage,
  unlockSeasonEarly,
  waterPlot,
  type GameState,
  type SeedKind,
} from './game/simulation';
import {
  getStageConfig,
  type AlchemyStageId,
  type ChallengeStageId,
} from './game/alchemyStages';
import { TitleScreen } from './components/TitleScreen';
import { TopHud } from './components/TopHud';
import { GreenhouseBench } from './components/GreenhouseBench';
import { AlembicWorkshop } from './components/AlembicWorkshop';
import { VillagerCounter } from './components/VillagerCounter';
import { ShopDrawer, type ShopTabId } from './components/ShopDrawer';
import { GrimoireModal } from './components/GrimoireModal';
import { AnalysisModal } from './components/AnalysisModal';
import { PauseModal } from './components/PauseModal';
import { MobileTabBar, type MainTab } from './components/MobileTabBar';

type ShellMode = 'title' | 'playing' | 'paused';
type ActiveOverlay = 'none' | 'shop' | 'grimoire' | 'analysis';

function getInitialShellMode(): ShellMode {
  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    if (params.get('autostart') === '1') {
      return 'playing';
    }
  }
  return 'title';
}

export default function App() {
  const [shellMode, setShellMode] = useState<ShellMode>(getInitialShellMode);
  const [overlay, setOverlay] = useState<ActiveOverlay>('none');
  const [shopTab, setShopTab] = useState<ShopTabId>('pots');
  const [muted, setMuted] = useState(false);
  const [activeTab, setActiveTab] = useState<MainTab>('bench');
  // Focus mode: the alchemy panel fills the screen; the HUD and tab bar hide.
  const [focusMode, setFocusMode] = useState(false);

  const [game, setGame] = useState<GameState>(() => createInitialGameState());
  const [selectedInitial, setSelectedInitial] = useState<string>('j');
  const [selectedMedial, setSelectedMedial] = useState<MedialLetter>('i');
  const [selectedFinal, setSelectedFinal] = useState<string>('a');
  const [selectedTone, setSelectedTone] = useState<ToneNumber>(1);

  // Sync game shell state to document.body.dataset.state
  useEffect(() => {
    document.body.dataset.state = shellMode;
  }, [shellMode]);

  const handleToggleMute = useCallback(() => {
    setMuted((prev) => {
      const next = !prev;
      soundEngine.setMuted(next);
      return next;
    });
  }, []);

  // Global keyboard shortcuts (M mute, P / Escape pause or close overlay)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }
      if (e.key === 'm' || e.key === 'M') {
        handleToggleMute();
        return;
      }
      if (e.key === 'Escape' || e.key === 'p' || e.key === 'P') {
        if (overlay !== 'none') {
          setOverlay('none');
          return;
        }
        if (shellMode === 'playing') {
          setShellMode('paused');
        } else if (shellMode === 'paused') {
          setShellMode('playing');
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [shellMode, overlay, handleToggleMute]);

  const handleStartGame = useCallback(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
    document.body.dataset.state = 'playing';
    setOverlay('none');
    setShellMode('playing');
  }, []);

  const handleRestart = useCallback(() => {
    setGame(createInitialGameState());
    setSelectedInitial('j');
    setSelectedMedial('i');
    setSelectedFinal('a');
    setSelectedTone(1);
    setOverlay('none');
    setShellMode('playing');
  }, []);

  const handleWaterPlot = useCallback((plotId: number) => {
    soundEngine.playSfx('water');
    setGame((prev) => waterPlot(prev, plotId));
  }, []);

  const handleHarvestPlot = useCallback((plotId: number) => {
    soundEngine.playSfx('harvest');
    setGame((prev) => harvestPlot(prev, plotId));
  }, []);

  const handlePlantSeed = useCallback((plotId: number, seedId: string, kind: SeedKind) => {
    soundEngine.playSfx('plant');
    setGame((prev) => plantSeed(prev, plotId, seedId, kind));
  }, []);

  const handleClearPlot = useCallback((plotId: number) => {
    setGame((prev) => clearPlot(prev, plotId));
  }, []);

  const handleSpeakSeed = useCallback((seedId: string) => {
    void soundEngine.playPinyin(seedId);
  }, []);

  const handleSelectStage = useCallback((stageId: AlchemyStageId) => {
    soundEngine.playSfx('plant');
    const stage = getStageConfig(stageId);
    if (!stage.allowMedial) {
      setSelectedMedial('');
    }
    if (stageId === 'free') {
      setSelectedInitial('j');
      setSelectedMedial('i');
      setSelectedFinal('a');
      setSelectedTone(1);
    } else {
      // Clear current slots so player intentionally builds the stage target
      setSelectedInitial('');
      setSelectedMedial('');
      setSelectedFinal('');
      setSelectedTone(1);
    }
    setGame((prev) => {
      const next = selectAlchemyStage(prev, stageId);
      if (stageId !== 'free' && stage.blindMode) {
        const step = next.stageProgress[stageId] ?? 0;
        const task = stage.tasks[step];
        if (task) {
          void soundEngine.playPinyin(task.syllable, task.tone);
        }
      }
      return next;
    });
  }, []);

  const handleReplayStage = useCallback((stageId: ChallengeStageId) => {
    soundEngine.playSfx('plant');
    setSelectedInitial('');
    setSelectedMedial('');
    setSelectedFinal('');
    setSelectedTone(1);
    setGame((prev) => replayAlchemyStage(prev, stageId));
  }, []);

  const handleBrew = useCallback(() => {
    setGame((prev) => {
      const result = brewPotion(
        prev,
        selectedInitial,
        selectedFinal,
        selectedTone,
        selectedMedial
      );
      if (result.outcome.valid && result.toneSupported) {
        soundEngine.playSfx(result.stageMatched ? 'coin' : 'brew_ok');
        void soundEngine.playPinyin(result.outcome.syllable, selectedTone);
      } else {
        soundEngine.playSfx('brew_fail');
      }
      return result.state;
    });
  }, [selectedInitial, selectedMedial, selectedFinal, selectedTone]);

  const handleFulfillOrder = useCallback((orderId: string) => {
    setGame((prev) => {
      const order = prev.orders.find((o) => o.id === orderId);
      const next = fulfillOrder(prev, orderId);
      if (next.ordersCompleted > prev.ordersCompleted) {
        soundEngine.playSfx('coin');
        if (order) {
          void soundEngine.playPinyin(order.targetSyllable, order.targetTone);
        }
      }
      return next;
    });
  }, []);

  const handleRevealHint = useCallback((orderId: string) => {
    setGame((prev) => revealOrderHint(prev, orderId));
  }, []);

  const handleAdvanceDay = useCallback(() => {
    soundEngine.playSfx('harvest');
    setGame((prev) => advanceDay(prev));
  }, []);

  const handleBuyDecor = useCallback((id: DecorId) => {
    soundEngine.playSfx('coin');
    setGame((prev) => buyDecor(prev, id));
  }, []);

  const handleSelectTheme = useCallback((id: PotThemeId) => {
    soundEngine.playSfx('plant');
    setGame((prev) => buyOrSelectPotTheme(prev, id));
  }, []);

  const handleUnlockSeason = useCallback((seasonIndex: number) => {
    soundEngine.playSfx('brew_ok');
    setGame((prev) => unlockSeasonEarly(prev, seasonIndex));
  }, []);

  const handleExpandPlot = useCallback(() => {
    soundEngine.playSfx('coin');
    setGame((prev) => expandGreenhousePlot(prev));
  }, []);

  const handleBuySeedPack = useCallback((seedId: string, kind: SeedKind) => {
    soundEngine.playSfx('coin');
    void soundEngine.playPinyin(seedId);
    setGame((prev) => buySeedPack(prev, seedId, kind));
  }, []);

  const handleQuickPlant = useCallback((seedId: string, kind: SeedKind) => {
    soundEngine.playSfx('plant');
    void soundEngine.playPinyin(seedId);
    setGame((prev) => quickPlantToEmptyPlot(prev, seedId, kind));
  }, []);

  const openShopOnTab = useCallback((tab: ShopTabId) => {
    setShopTab(tab);
    setOverlay('shop');
  }, []);

  if (shellMode === 'title') {
    return (
      <>
        <TitleScreen
          onStart={handleStartGame}
          onOpenAnalysis={() => setOverlay('analysis')}
          muted={muted}
          onToggleMute={handleToggleMute}
        />
        {overlay === 'analysis' && <AnalysisModal onClose={() => setOverlay('none')} />}
      </>
    );
  }

  // The three functional zones are shared between the mobile tabbed layout
  // and the desktop three-column layout.
  const bench = (
    <GreenhouseBench
      plots={game.plots}
      potTheme={game.potTheme}
      unlockedInitials={game.unlockedInitials}
      unlockedFinals={game.unlockedFinals}
      pantry={game.pantry}
      energy={game.energy}
      coins={game.coins}
      hasAutoSprinkler={game.ownedDecors.includes('auto_sprinkler')}
      onWaterPlot={handleWaterPlot}
      onHarvestPlot={handleHarvestPlot}
      onPlantSeed={handlePlantSeed}
      onClearPlot={handleClearPlot}
      onExpandPlot={handleExpandPlot}
      onSpeakSeed={handleSpeakSeed}
    />
  );

  const workshop = (
    <AlembicWorkshop
      unlockedInitials={game.unlockedInitials}
      unlockedFinals={game.unlockedFinals}
      pantry={game.pantry}
      selectedInitial={selectedInitial}
      selectedMedial={selectedMedial}
      selectedFinal={selectedFinal}
      selectedTone={selectedTone}
      hasEmptyPot={game.plots.some((p) => p.seedId === null)}
      activeStageId={game.activeStageId}
      stageProgress={game.stageProgress}
      clearedStages={game.clearedStages}
      onSelectStage={handleSelectStage}
      onReplayStage={handleReplayStage}
      onSelectInitial={(val) => {
        setSelectedInitial(val);
        if (val) void soundEngine.playPinyin(val);
      }}
      onSelectMedial={(val) => {
        setSelectedMedial(val);
        if (val) void soundEngine.playPinyin(val);
      }}
      onSelectFinal={(val) => {
        setSelectedFinal(val);
        void soundEngine.playPinyin(val);
      }}
      onSelectTone={setSelectedTone}
      onBrew={handleBrew}
      onClearSelection={() => {
        setSelectedInitial('');
        setSelectedMedial('');
        setSelectedFinal('');
        setSelectedTone(1);
      }}
      onOpenSeedShop={() => openShopOnTab('seeds')}
      onQuickPlant={handleQuickPlant}
      onPlayPotion={(syllable, tone, slow) =>
        void soundEngine.playPinyin(syllable, tone, { slow })
      }
      potions={game.potions}
      lastBrew={game.lastBrew}
      focus={focusMode}
      onToggleFocus={() => setFocusMode((v) => !v)}
    />
  );

  const villager = (
    <VillagerCounter
      orders={game.orders}
      potions={game.potions}
      hasEchoBell={game.ownedDecors.includes('owl_echo_bell')}
      onListenOrder={(syllable, tone, slow) =>
        void soundEngine.playPinyin(syllable, tone, { slow })
      }
      onRevealHint={handleRevealHint}
      onFulfillOrder={handleFulfillOrder}
    />
  );

  return (
    <div className="flex h-dvh flex-col overflow-hidden bg-[#ECE7D9] text-[#232B23]">
      {/* Safe-area strips: the nsh host injects the device's real insets, so the
          HUD never sits under the status bar and the tab bar never under the
          gesture bar. The strips carry the same dark tone as the chrome. */}
      <div
        className={`shrink-0 ${focusMode ? 'bg-[#FAF7EE]' : 'bg-[#212D23]'}`}
        style={{ height: 'var(--safe-top)' }}
      />

      {!focusMode && (
        <TopHud
          state={game}
          muted={muted}
          onAdvanceDay={handleAdvanceDay}
          onOpenShop={() => openShopOnTab('pots')}
          onOpenGrimoire={() => setOverlay('grimoire')}
          onOpenAnalysis={() => setOverlay('analysis')}
          onToggleMute={handleToggleMute}
          onPause={() => setShellMode('paused')}
        />
      )}

      {/* Quiet In-World Status Whisper Bar — desktop only; on phones each
          panel already shows its own context, and every dp counts there */}
      <div className="hidden shrink-0 border-b border-[#D8D0BE] bg-[#F5F1E5] px-4 py-2 sm:px-6 lg:block">
        <div
          data-testid="status-banner"
          className="mx-auto flex max-w-[1420px] items-center justify-between gap-4 text-xs text-[#3D473E]"
        >
          <span className="truncate font-medium">{game.statusBanner}</span>
          <span className="hidden shrink-0 text-[#6B7166] lg:inline">
            已完成委托 <strong className="font-mono-num text-[#235840]">{game.ordersCompleted}</strong> 份 · 真实发音来自 flashtony2005/pinyin
          </span>
        </div>
      </div>

      {/* Mobile: one zone at a time, shown by the bottom tab bar. The safe-area
          strips above/below already keep content clear of the system bars. */}
      <main
        className={`min-h-0 flex-1 lg:hidden ${focusMode ? 'p-1' : 'p-1.5'}`}
      >
        <div className="h-full">{activeTab === 'bench' ? bench : activeTab === 'workshop' ? workshop : villager}</div>
      </main>

      {/* Desktop: the original three-column greenhouse stage (hidden in focus mode) */}
      {!focusMode && (
      <main className="mx-auto hidden w-full max-w-[1420px] flex-1 grid-cols-12 gap-5 overflow-hidden p-4 sm:p-6 lg:grid">
        <div className="min-h-0 lg:col-span-4">{bench}</div>
        <div className="min-h-0 lg:col-span-5">{workshop}</div>
        <div className="min-h-0 lg:col-span-3">{villager}</div>
      </main>
      )}

      {!focusMode && (
        <MobileTabBar
          active={activeTab}
          onChange={setActiveTab}
          orderCount={game.orders.length}
        />
      )}

      {/* Bottom safe-area strip (the tab bar above it is hidden in focus mode) */}
      <div
        className={`shrink-0 ${focusMode ? 'bg-[#FAF7EE]' : 'bg-[#212D23]'}`}
        style={{ height: 'var(--safe-bottom)' }}
      />

      {/* Modals & Drawers */}
      {overlay === 'shop' && (
        <ShopDrawer
          state={game}
          initialTab={shopTab}
          onClose={() => setOverlay('none')}
          onBuyDecor={handleBuyDecor}
          onSelectTheme={handleSelectTheme}
          onUnlockSeason={handleUnlockSeason}
          onExpandPlot={handleExpandPlot}
          onBuySeedPack={handleBuySeedPack}
          onQuickPlant={handleQuickPlant}
        />
      )}

      {overlay === 'grimoire' && (
        <GrimoireModal
          discoveredSyllables={game.discoveredSyllables}
          discoveredTones={game.discoveredTones}
          onPlayAudio={(syllable, tone) => void soundEngine.playPinyin(syllable, tone)}
          onClose={() => setOverlay('none')}
        />
      )}

      {overlay === 'analysis' && <AnalysisModal onClose={() => setOverlay('none')} />}

      {shellMode === 'paused' && (
        <PauseModal
          state={game}
          onResume={() => setShellMode('playing')}
          onRestart={handleRestart}
          onReturnTitle={() => {
            setOverlay('none');
            setShellMode('title');
          }}
        />
      )}
    </div>
  );
}
