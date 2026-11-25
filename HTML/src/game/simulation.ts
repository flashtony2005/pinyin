import {
  DECOR_CATALOG,
  POT_THEMES,
  SEASONS,
  VILLAGERS,
  type DecorId,
  type MedialLetter,
  type PotThemeId,
  type SeasonId,
  type ToneNumber,
} from '../data/pinyinResources';
import {
  applyToneMark,
  composePinyin,
  getAvailableTones,
  getExampleChar,
  getPotionColorHex,
  type ComposeOutcome,
} from './pinyinEngine';
import {
  getStageConfig,
  type AlchemyStageId,
  type ChallengeStageId,
} from './alchemyStages';

export type SeedKind = 'initial' | 'final';

export interface PlotState {
  id: number;
  seedId: string | null;
  kind: SeedKind | null;
  /** 0 = sprout, 1 = budding, 2 = mature & ready to harvest */
  growth: 0 | 1 | 2;
  wateredToday: boolean;
}

export interface BrewedPotion {
  id: string;
  syllable: string;
  tone: ToneNumber;
  marked: string;
  initial: string;
  medial: MedialLetter;
  final: string;
  isTriplet: boolean;
  char: string;
  word: string;
  colorHex: string;
  count: number;
}

export type OrderMode = 'listen' | 'character' | 'tone_rule';

export interface VillagerOrder {
  id: string;
  villagerId: string;
  villagerName: string;
  villagerRole: string;
  avatar: string;
  mode: OrderMode;
  targetInitial: string;
  targetMedial: MedialLetter;
  targetFinal: string;
  targetSyllable: string;
  targetTone: ToneNumber;
  targetMarked: string;
  targetChar: string;
  targetWord: string;
  promptText: string;
  rewardGold: number;
  hintRevealed: boolean;
}

export interface LastBrewFeedback {
  id: number;
  success: boolean;
  syllable: string;
  marked: string;
  tone: ToneNumber;
  char: string;
  word: string;
  isFirstDiscovery: boolean;
  message: string;
  ruleNote: string;
}

export interface GameState {
  day: number;
  seasonIndex: number;
  coins: number;
  energy: number;
  maxEnergy: number;
  ordersCompleted: number;
  potTheme: PotThemeId;
  unlockedThemes: PotThemeId[];
  ownedDecors: DecorId[];
  unlockedInitials: string[];
  unlockedFinals: string[];
  /** Pantry ingredient counts keyed by letter (e.g. 'b': 3, 'a': 4) */
  pantry: Record<string, number>;
  plots: PlotState[];
  potions: BrewedPotion[];
  /** Unique discovered syllable keys (e.g. 'ba', 'ma', 'ju') */
  discoveredSyllables: string[];
  /** Unique discovered syllable+tone keys (e.g. 'ba:1') */
  discoveredTones: string[];
  orders: VillagerOrder[];
  nextOrderSeq: number;
  lastBrew: LastBrewFeedback | null;
  brewSeq: number;
  statusBanner: string;
  activeStageId: AlchemyStageId;
  stageProgress: Record<ChallengeStageId, number>;
  clearedStages: ChallengeStageId[];
}

export interface OrderTemplate {
  initial: string;
  medial?: MedialLetter;
  final: string;
  tone: ToneNumber;
  mode: OrderMode;
  promptBuilder: (char: string, word: string, marked: string) => string;
  reward: number;
  minSeason: number;
}

const ORDER_CURRICULUM: OrderTemplate[] = [
  {
    initial: 'm',
    final: 'a',
    tone: 3,
    mode: 'character',
    promptBuilder: (char, word) => `帮我调制呼唤「${char}（${word}）」的温润药水吧！`,
    reward: 25,
    minSeason: 0,
  },
  {
    initial: 'j',
    medial: 'i',
    final: 'a',
    tone: 1,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `试试三拼音节吧！用声母 j + 介母 i + 韵母 a 炼出「${char}（${word}）」药水。`,
    reward: 35,
    minSeason: 0,
  },
  {
    initial: 'b',
    final: 'a',
    tone: 1,
    mode: 'listen',
    promptBuilder: () => '听听我带来的森林回音，帮我配出同样读音的药水！',
    reward: 30,
    minSeason: 0,
  },
  {
    initial: 'l',
    final: 'ü',
    tone: 4,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `我需要保留小鱼两点的「${char}（${word}）」色叶露（声母 l + 韵母 ü）！`,
    reward: 35,
    minSeason: 0,
  },
  {
    initial: 'd',
    medial: 'u',
    final: 'o',
    tone: 3,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `需要三拼音节「${char}（${word}）」露水，试试 d + u + o！`,
    reward: 36,
    minSeason: 0,
  },
  {
    initial: 'f',
    final: 'a',
    tone: 1,
    mode: 'character',
    promptBuilder: (char, word) => `温室春雨将至，请配一瓶助苗「${char}（${word}）」的露水。`,
    reward: 28,
    minSeason: 0,
  },
  {
    initial: 'd',
    final: 'i',
    tone: 1,
    mode: 'listen',
    promptBuilder: () => '点一点小喇叭，听清声母与声调再开始炼制哦！',
    reward: 30,
    minSeason: 0,
  },
  {
    initial: 'm',
    final: 'i',
    tone: 4,
    mode: 'character',
    promptBuilder: (char, word) => `想给邻居送一瓶甜甜的「${char}（${word}）」香药水。`,
    reward: 30,
    minSeason: 0,
  },
  {
    initial: 't',
    final: 'u',
    tone: 3,
    mode: 'listen',
    promptBuilder: () => '听一听这段声调拐弯的三声密码，配一瓶给我吧！',
    reward: 32,
    minSeason: 0,
  },
  {
    initial: 'n',
    final: 'ü',
    tone: 3,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `请用保留两点的规则调配「${char}（${word}）」之露（n + ü）。`,
    reward: 36,
    minSeason: 0,
  },
  {
    initial: 'j',
    final: 'ü',
    tone: 1,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `考考炼金术士：j 遇到小鱼 ü 要去掉两点！请调出「${char}（${word}）」药水。`,
    reward: 42,
    minSeason: 1,
  },
  {
    initial: 'h',
    final: 'u',
    tone: 2,
    mode: 'character',
    promptBuilder: (char, word) => `洛洛想画一只灵动的「${char}（${word}）」，需要对应读音的颜料露。`,
    reward: 38,
    minSeason: 1,
  },
  {
    initial: 'k',
    final: 'ai',
    tone: 1,
    mode: 'listen',
    promptBuilder: () => '这是夏日复韵母的歌声，听完帮我炼一瓶吧！',
    reward: 42,
    minSeason: 1,
  },
  {
    initial: 'h',
    final: 'ao',
    tone: 3,
    mode: 'character',
    promptBuilder: (char, word) => `为全村带来「${char}（${word}）」的三声复韵母药水！`,
    reward: 40,
    minSeason: 1,
  },
  {
    initial: 'h',
    medial: 'u',
    final: 'a',
    tone: 1,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `夏日三拼挑战：用 h + u + a 炼出盛放的「${char}（${word}）」之露！`,
    reward: 44,
    minSeason: 1,
  },
  {
    initial: 'g',
    medial: 'u',
    final: 'a',
    tone: 1,
    mode: 'character',
    promptBuilder: (char, word) => `请用三拼音节调一瓶清甜的「${char}（${word}）」果露。`,
    reward: 42,
    minSeason: 1,
  },
  {
    initial: 'q',
    final: 'ü',
    tone: 3,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `q 遇到 ü 也会触发去点魔法！帮我酿一瓶「${char}（${word}）」调魔药。`,
    reward: 45,
    minSeason: 1,
  },
  {
    initial: 'sh',
    final: 'u',
    tone: 1,
    mode: 'listen',
    promptBuilder: () => '注意分辨翘舌音哦！听声音帮我配出正确药水。',
    reward: 48,
    minSeason: 2,
  },
  {
    initial: 'zh',
    final: 'i',
    tone: 4,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `这是一瓶整体认读的「${char}（${word}）」愈魔药，注意翘舌音 zh！`,
    reward: 50,
    minSeason: 2,
  },
  {
    initial: 'l',
    final: 'an',
    tone: 2,
    mode: 'character',
    promptBuilder: (char, word) => `温室花房需要一瓶前鼻音「${char}（${word}）」花露。`,
    reward: 48,
    minSeason: 2,
  },
  {
    initial: 'q',
    final: 'ing',
    tone: 1,
    mode: 'listen',
    promptBuilder: () => '听听后鼻音的清脆回响，炼出这瓶秋日露水吧！',
    reward: 52,
    minSeason: 2,
  },
  {
    initial: 'y',
    final: 'ü',
    tone: 2,
    mode: 'tone_rule',
    promptBuilder: (char, word) =>
      `y 遇到 ü 同样去点！帮我调制荷塘「${char}（${word}）」儿戏水露。`,
    reward: 55,
    minSeason: 3,
  },
];

export function buildOrderFromIndex(seq: number, seasonIndex: number): VillagerOrder {
  const eligible = ORDER_CURRICULUM.filter((o) => o.minSeason <= seasonIndex);
  const template = eligible[seq % eligible.length];
  const villager = VILLAGERS[seq % VILLAGERS.length];
  const medial: MedialLetter = template.medial ?? '';
  const composed = composePinyin(template.initial, template.final, medial);
  const syllable = composed.valid
    ? composed.syllable
    : `${template.initial}${medial}${template.final}`;
  const marked = applyToneMark(syllable, template.tone);
  const { char, word } = getExampleChar(syllable, template.tone);

  return {
    id: `order_${seq}`,
    villagerId: villager.id,
    villagerName: villager.name,
    villagerRole: villager.role,
    avatar: villager.avatar,
    mode: template.mode,
    targetInitial: template.initial,
    targetMedial: medial,
    targetFinal: template.final,
    targetSyllable: syllable,
    targetTone: template.tone,
    targetMarked: marked,
    targetChar: char,
    targetWord: word,
    promptText: template.promptBuilder(char, word, marked),
    rewardGold: template.reward,
    hintRevealed: false,
  };
}

export function createInitialGameState(): GameState {
  const spring = SEASONS[0];
  const initialOrders: VillagerOrder[] = [
    buildOrderFromIndex(0, 0),
    buildOrderFromIndex(1, 0),
    buildOrderFromIndex(2, 0),
  ];

  return {
    day: 1,
    seasonIndex: 0,
    coins: 40,
    energy: 6,
    maxEnergy: 6,
    ordersCompleted: 0,
    potTheme: 'terracotta',
    unlockedThemes: ['terracotta'],
    ownedDecors: [],
    unlockedInitials: [...spring.initials],
    unlockedFinals: [...spring.finals],
    pantry: {
      b: 3,
      m: 3,
      l: 2,
      f: 2,
      d: 2,
      j: 3,
      q: 2,
      x: 2,
      a: 4,
      o: 2,
      i: 3,
      u: 2,
      ü: 2,
    },
    plots: [
      { id: 0, seedId: 'b', kind: 'initial', growth: 2, wateredToday: true },
      { id: 1, seedId: 'a', kind: 'final', growth: 1, wateredToday: false },
      { id: 2, seedId: 'm', kind: 'initial', growth: 2, wateredToday: true },
      { id: 3, seedId: 'ü', kind: 'final', growth: 0, wateredToday: false },
      { id: 4, seedId: 'j', kind: 'initial', growth: 1, wateredToday: false },
      { id: 5, seedId: null, kind: null, growth: 0, wateredToday: false },
    ],
    potions: [],
    discoveredSyllables: [],
    discoveredTones: [],
    orders: initialOrders,
    nextOrderSeq: 3,
    lastBrew: null,
    brewSeq: 0,
    statusBanner: '欢迎来到魔法拼音温室！点击花盆浇水收获，或在炼金台拼出声调魔药。',
    activeStageId: 'free',
    stageProgress: { lvl1: 0, lvl2: 0, lvl3: 0, lvl4: 0 },
    clearedStages: [],
  };
}

export function plantSeed(
  state: GameState,
  plotId: number,
  seedId: string,
  kind: SeedKind
): GameState {
  const unlocked =
    kind === 'initial'
      ? state.unlockedInitials.includes(seedId)
      : state.unlockedFinals.includes(seedId);
  if (!unlocked) return state;

  const plots = state.plots.map((plot) =>
    plot.id === plotId
      ? { ...plot, seedId, kind, growth: 0 as const, wateredToday: false }
      : plot
  );
  return {
    ...state,
    plots,
    statusBanner: `已在 ${plotId + 1} 号花盆种下${kind === 'initial' ? '声母晶芽' : '韵母灵花'}「${seedId}」。`,
  };
}

export function clearPlot(state: GameState, plotId: number): GameState {
  const plots = state.plots.map((plot) =>
    plot.id === plotId
      ? { ...plot, seedId: null, kind: null, growth: 0 as const, wateredToday: false }
      : plot
  );
  return {
    ...state,
    plots,
    statusBanner: `已清空 ${plotId + 1} 号花盆，可以重新播种新拼音植物。`,
  };
}

export function waterPlot(state: GameState, plotId: number): GameState {
  const target = state.plots.find((p) => p.id === plotId);
  if (!target || !target.seedId) return state;
  if (target.growth >= 2) {
    return { ...state, statusBanner: `「${target.seedId}」已经成熟盛放，点击即可收获！` };
  }
  if (state.energy <= 0) {
    return { ...state, statusBanner: '今日精力用完啦！点击右上角「结束今天」休息恢复精力。' };
  }

  const nextGrowth = Math.min(2, target.growth + 1) as 0 | 1 | 2;
  const plots = state.plots.map((p) =>
    p.id === plotId ? { ...p, growth: nextGrowth, wateredToday: true } : p
  );

  return {
    ...state,
    energy: state.energy - 1,
    plots,
    statusBanner:
      nextGrowth === 2
        ? `「${target.seedId}」喝饱魔法露水，已经成熟可以收获啦！`
        : `为「${target.seedId}」浇了水，嫩芽长高了一截！`,
  };
}

export function harvestPlot(state: GameState, plotId: number): GameState {
  const target = state.plots.find((p) => p.id === plotId);
  if (!target || !target.seedId || target.growth < 2) return state;

  const bonus = state.ownedDecors.includes('star_fertilizer') ? 2 : 0;
  const yieldAmount = 3 + bonus;
  const nextPantry = {
    ...state.pantry,
    [target.seedId]: (state.pantry[target.seedId] ?? 0) + yieldAmount,
  };

  // Cozy perennial regrow to stage 1 so tending feels rewarding and never tedious
  const plots = state.plots.map((p) =>
    p.id === plotId ? { ...p, growth: 1 as const, wateredToday: false } : p
  );

  return {
    ...state,
    pantry: nextPantry,
    plots,
    statusBanner: `收获成功！获得「${target.seedId}」×${yieldAmount}，已存入炼金素材架。`,
  };
}

export function brewPotion(
  state: GameState,
  initial: string,
  final_: string,
  tone: ToneNumber,
  medial: MedialLetter = ''
): { state: GameState; outcome: ComposeOutcome; toneSupported: boolean; stageMatched?: boolean } {
  const stage = getStageConfig(state.activeStageId);
  const isChallengeStage = state.activeStageId !== 'free';
  const cleanInitial = initial.trim();
  const cleanFinal = final_.trim();
  const cleanMedial = (stage.allowMedial ? (medial || '').trim() : '') as MedialLetter;

  if (!cleanFinal) {
    return {
      state: { ...state, statusBanner: '请先选择一株韵母灵花放入蒸馏台！' },
      outcome: composePinyin(cleanInitial, cleanFinal, cleanMedial),
      toneSupported: false,
    };
  }

  // In Free Mode, enforce pantry stock; in Challenge Stages, allow free trial supply so players never get stuck
  if (!isChallengeStage) {
    if (cleanInitial && (state.pantry[cleanInitial] ?? 0) <= 0) {
      return {
        state: { ...state, statusBanner: `声母「${cleanInitial}」库存不足，可去温室收获或在商店补货！` },
        outcome: composePinyin(cleanInitial, cleanFinal, cleanMedial),
        toneSupported: false,
      };
    }
    if (cleanMedial && (state.pantry[cleanMedial] ?? 0) <= 0) {
      return {
        state: { ...state, statusBanner: `介母「${cleanMedial}」库存不足，可去温室收获或在商店补货！` },
        outcome: composePinyin(cleanInitial, cleanFinal, cleanMedial),
        toneSupported: false,
      };
    }
    if ((state.pantry[cleanFinal] ?? 0) <= 0) {
      return {
        state: { ...state, statusBanner: `韵母「${cleanFinal}」库存不足，可去温室收获或在商店补货！` },
        outcome: composePinyin(cleanInitial, cleanFinal, cleanMedial),
        toneSupported: false,
      };
    }
  }

  const outcome = composePinyin(cleanInitial, cleanFinal, cleanMedial);
  const nextBrewSeq = state.brewSeq + 1;

  if (!outcome.valid) {
    const formulaText = cleanMedial
      ? `${cleanInitial || '零'} + ${cleanMedial} + ${cleanFinal}`
      : `${cleanInitial || '零'} + ${cleanFinal}`;
    return {
      state: {
        ...state,
        brewSeq: nextBrewSeq,
        lastBrew: {
          id: nextBrewSeq,
          success: false,
          syllable: `${cleanInitial}${cleanMedial}${cleanFinal}`,
          marked: formulaText,
          tone,
          char: '',
          word: '',
          isFirstDiscovery: false,
          message: '炼金罐冒出一团紫色泡泡——这不是合法的普通话音节！',
          ruleNote: outcome.reason ?? '请更换声母、介母或韵母组合再试试。',
        },
        statusBanner: outcome.reason ?? '非法拼音组合，素材已贴心保留！',
      },
      outcome,
      toneSupported: false,
    };
  }

  const availableTones = getAvailableTones(outcome.syllable);
  const toneSupported = availableTones.includes(tone);
  const marked = applyToneMark(outcome.syllable, tone);

  if (!toneSupported) {
    const validStr = availableTones.map((t) => `${t}声`).join(' / ');
    return {
      state: {
        ...state,
        brewSeq: nextBrewSeq,
        lastBrew: {
          id: nextBrewSeq,
          success: false,
          syllable: outcome.syllable,
          marked,
          tone,
          char: '',
          word: '',
          isFirstDiscovery: false,
          message: `普通话中音节「${outcome.syllable}」没有常用 ${tone} 声字哦！`,
          ruleNote: `试试切换火候为：${validStr}`,
        },
        statusBanner: `音节「${outcome.syllable}」常用声调为 ${validStr}，已保留素材。`,
      },
      outcome,
      toneSupported: false,
    };
  }

  // Valid brew! Consume pantry ingredients in Free Mode; Challenge Stages use trial supply
  const nextPantry = { ...state.pantry };
  if (!isChallengeStage) {
    if (cleanInitial) {
      nextPantry[cleanInitial] = Math.max(0, (nextPantry[cleanInitial] ?? 1) - 1);
    }
    if (cleanMedial) {
      nextPantry[cleanMedial] = Math.max(0, (nextPantry[cleanMedial] ?? 1) - 1);
    }
    nextPantry[cleanFinal] = Math.max(0, (nextPantry[cleanFinal] ?? 1) - 1);
  }

  const potionId = `${outcome.syllable}:${tone}`;
  const { char, word } = getExampleChar(outcome.syllable, tone);
  const colorHex = getPotionColorHex(outcome.syllable, tone);

  const existing = state.potions.find((p) => p.id === potionId);
  const nextPotions = existing
    ? state.potions.map((p) => (p.id === potionId ? { ...p, count: p.count + 1 } : p))
    : [
        {
          id: potionId,
          syllable: outcome.syllable,
          tone,
          marked,
          initial: cleanInitial,
          medial: cleanMedial,
          final: cleanFinal,
          isTriplet: outcome.isTriplet,
          char,
          word,
          colorHex,
          count: 1,
        },
        ...state.potions,
      ];

  const isFirstDiscovery = !state.discoveredSyllables.includes(outcome.syllable);
  const discoveredSyllables = isFirstDiscovery
    ? [...state.discoveredSyllables, outcome.syllable]
    : state.discoveredSyllables;
  const discoveredTones = state.discoveredTones.includes(potionId)
    ? state.discoveredTones
    : [...state.discoveredTones, potionId];
  const discoveryBonus = isFirstDiscovery ? (outcome.isTriplet ? 15 : 12) : 0;

  // Check challenge stage task progression
  let nextStageProgress = state.stageProgress;
  let nextClearedStages = state.clearedStages;
  let stageClearBonus = 0;
  let stageMatched = false;
  let stageFeedbackMsg: string | null = null;

  if (isChallengeStage) {
    const stageId = state.activeStageId as ChallengeStageId;
    const step = state.stageProgress[stageId] ?? 0;
    const currentTask = stage.tasks[step];
    if (currentTask) {
      if (currentTask.syllable === outcome.syllable && currentTask.tone === tone) {
        stageMatched = true;
        const nextStep = step + 1;
        nextStageProgress = { ...state.stageProgress, [stageId]: nextStep };
        if (nextStep >= stage.tasks.length) {
          const alreadyCleared = state.clearedStages.includes(stageId);
          if (!alreadyCleared) {
            nextClearedStages = [...state.clearedStages, stageId];
            stageClearBonus = stage.rewardGold;
            stageFeedbackMsg = `通关【${stage.title}】！首次通关奖励 +${stageClearBonus} 金币`;
          } else {
            stageFeedbackMsg = `再次通关【${stage.title}】！全部 ${stage.tasks.length} 题精准完成`;
          }
        } else {
          stageFeedbackMsg = `关卡试炼通过 (${nextStep}/${stage.tasks.length})！继续下一题吧`;
        }
      } else {
        const targetHint = stage.blindMode
          ? '与本关听音目标不同，点一下「听目标音」再辨一辨声韵与声调吧！'
          : `本关当前目标为「${applyToneMark(currentTask.syllable, currentTask.tone)}」，试试调整声母/介母/声调！`;
        stageFeedbackMsg = `炼出了有效魔药「${marked}（${char}）」，但${targetHint}`;
      }
    }
  }

  const totalBonus = discoveryBonus + stageClearBonus;
  const defaultSuccessMsg = isFirstDiscovery
    ? `发现新${outcome.isTriplet ? '三拼' : '音节'}魔药「${marked}（${char}）」！药典收录奖励 +${discoveryBonus} 金币`
    : `炼成魔药「${marked}（${char}）」！已上架到药水柜。`;

  return {
    state: {
      ...state,
      coins: state.coins + totalBonus,
      pantry: nextPantry,
      potions: nextPotions,
      discoveredSyllables,
      discoveredTones,
      stageProgress: nextStageProgress,
      clearedStages: nextClearedStages,
      brewSeq: nextBrewSeq,
      lastBrew: {
        id: nextBrewSeq,
        success: true,
        syllable: outcome.syllable,
        marked,
        tone,
        char,
        word,
        isFirstDiscovery,
        message: stageFeedbackMsg ?? defaultSuccessMsg,
        ruleNote: outcome.ruleNote,
      },
      statusBanner:
        stageFeedbackMsg ??
        `炼制成功：${marked}（${char} · ${word}）${isFirstDiscovery ? ` · 新发现 +${discoveryBonus} 金币` : ''}`,
    },
    outcome,
    toneSupported: true,
    stageMatched,
  };
}

export function fulfillOrder(state: GameState, orderId: string): GameState {
  const order = state.orders.find((o) => o.id === orderId);
  if (!order) return state;

  const potionId = `${order.targetSyllable}:${order.targetTone}`;
  const matchingPotion = state.potions.find((p) => p.id === potionId && p.count > 0);
  if (!matchingPotion) {
    return {
      ...state,
      statusBanner: `药水架还没有「${order.targetMarked}」药水，先去炼金台调配吧！`,
    };
  }

  const nextPotions = state.potions
    .map((p) => (p.id === potionId ? { ...p, count: p.count - 1 } : p))
    .filter((p) => p.count > 0);

  const nextCompleted = state.ordersCompleted + 1;
  let nextSeasonIndex = state.seasonIndex;
  let unlockedInitials = [...state.unlockedInitials];
  let unlockedFinals = [...state.unlockedFinals];
  let seasonUnlockMsg = '';

  const nextSeason = SEASONS[state.seasonIndex + 1];
  if (nextSeason && nextCompleted >= nextSeason.requiredOrders) {
    nextSeasonIndex = state.seasonIndex + 1;
    for (const ini of nextSeason.initials) {
      if (!unlockedInitials.includes(ini)) unlockedInitials.push(ini);
    }
    for (const fin of nextSeason.finals) {
      if (!unlockedFinals.includes(fin)) unlockedFinals.push(fin);
    }
    seasonUnlockMsg = ` 季节更替：解锁【${nextSeason.name}】全部新种子！`;
  }

  const nextPantry = { ...state.pantry };
  if (seasonUnlockMsg && nextSeason) {
    // Gift 2 starter essences of newly unlocked seeds so player can experiment immediately
    for (const ini of nextSeason.initials.slice(0, 3)) {
      nextPantry[ini] = (nextPantry[ini] ?? 0) + 2;
    }
    for (const fin of nextSeason.finals.slice(0, 3)) {
      nextPantry[fin] = (nextPantry[fin] ?? 0) + 2;
    }
  }

  const replacement = buildOrderFromIndex(state.nextOrderSeq, nextSeasonIndex);
  const nextOrders = state.orders.map((o) => (o.id === orderId ? replacement : o));

  return {
    ...state,
    coins: state.coins + order.rewardGold,
    ordersCompleted: nextCompleted,
    seasonIndex: nextSeasonIndex,
    unlockedInitials,
    unlockedFinals,
    pantry: nextPantry,
    potions: nextPotions,
    orders: nextOrders,
    nextOrderSeq: state.nextOrderSeq + 1,
    statusBanner: `${order.villagerName}开心地收下了「${order.targetMarked}（${order.targetChar}）」！获得 +${order.rewardGold} 金币。${seasonUnlockMsg}`,
  };
}

export function revealOrderHint(state: GameState, orderId: string): GameState {
  const order = state.orders.find((o) => o.id === orderId);
  if (!order || order.hintRevealed) return state;

  const isFree = state.ownedDecors.includes('owl_echo_bell');
  const cost = isFree ? 0 : 5;
  if (state.coins < cost) {
    return { ...state, statusBanner: '金币不足 5 枚，无法购买回音提示。' };
  }

  return {
    ...state,
    coins: state.coins - cost,
    orders: state.orders.map((o) => (o.id === orderId ? { ...o, hintRevealed: true } : o)),
    statusBanner: isFree
      ? `猫头鹰风铃免费为你揭示了「${order.villagerName}」的拼读配方！`
      : `花费 5 金币揭示了拼读提示！`,
  };
}

export function advanceDay(state: GameState): GameState {
  const hasSprinkler = state.ownedDecors.includes('auto_sprinkler');
  const plots = state.plots.map((plot) => {
    if (!plot.seedId) return plot;
    if (hasSprinkler && plot.growth < 2) {
      return {
        ...plot,
        growth: Math.min(2, plot.growth + 1) as 0 | 1 | 2,
        wateredToday: true,
      };
    }
    return { ...plot, wateredToday: false };
  });

  return {
    ...state,
    day: state.day + 1,
    energy: state.maxEnergy,
    plots,
    statusBanner: hasSprinkler
      ? `第 ${state.day + 1} 天清晨：精力已回满，铜制自动洒水器已自动浇灌全部幼苗！`
      : `第 ${state.day + 1} 天清晨：精力已恢复至 ${state.maxEnergy}/${state.maxEnergy}，开启新的拼音炼金日！`,
  };
}

export function buyDecor(state: GameState, decorId: DecorId): GameState {
  if (state.ownedDecors.includes(decorId)) return state;
  const decor = DECOR_CATALOG.find((d) => d.id === decorId);
  if (!decor) return state;
  if (state.coins < decor.cost) {
    return { ...state, statusBanner: `金币不足，购买「${decor.name}」需要 ${decor.cost} 金币。` };
  }

  const nextMaxEnergy = decorId === 'firefly_lantern' ? state.maxEnergy + 4 : state.maxEnergy;
  const nextEnergy = decorId === 'firefly_lantern' ? state.energy + 4 : state.energy;

  return {
    ...state,
    coins: state.coins - decor.cost,
    maxEnergy: nextMaxEnergy,
    energy: nextEnergy,
    ownedDecors: [...state.ownedDecors, decorId],
    statusBanner: `已布置「${decor.name}」！${decor.effect}`,
  };
}

export function buyOrSelectPotTheme(state: GameState, themeId: PotThemeId): GameState {
  const theme = POT_THEMES.find((t) => t.id === themeId);
  if (!theme) return state;

  if (state.unlockedThemes.includes(themeId)) {
    return {
      ...state,
      potTheme: themeId,
      statusBanner: `温室花盆已更换为「${theme.name}」。`,
    };
  }

  if (state.coins < theme.cost) {
    return { ...state, statusBanner: `金币不足，解锁「${theme.name}」需要 ${theme.cost} 金币。` };
  }

  return {
    ...state,
    coins: state.coins - theme.cost,
    potTheme: themeId,
    unlockedThemes: [...state.unlockedThemes, themeId],
    statusBanner: `已解锁并换上「${theme.name}」！`,
  };
}

export function unlockSeasonEarly(state: GameState, targetSeasonIndex: number): GameState {
  if (targetSeasonIndex <= state.seasonIndex || targetSeasonIndex >= SEASONS.length) return state;
  const season = SEASONS[targetSeasonIndex];
  const cost = 30 * targetSeasonIndex;
  if (state.coins < cost) {
    return { ...state, statusBanner: `提前解锁【${season.name}】需要 ${cost} 金币。` };
  }

  const unlockedInitials = [...state.unlockedInitials];
  const unlockedFinals = [...state.unlockedFinals];
  const nextPantry = { ...state.pantry };

  for (let i = state.seasonIndex + 1; i <= targetSeasonIndex; i++) {
    for (const ini of SEASONS[i].initials) {
      if (!unlockedInitials.includes(ini)) unlockedInitials.push(ini);
      nextPantry[ini] = (nextPantry[ini] ?? 0) + 2;
    }
    for (const fin of SEASONS[i].finals) {
      if (!unlockedFinals.includes(fin)) unlockedFinals.push(fin);
      nextPantry[fin] = (nextPantry[fin] ?? 0) + 2;
    }
  }

  return {
    ...state,
    coins: state.coins - cost,
    seasonIndex: targetSeasonIndex,
    unlockedInitials,
    unlockedFinals,
    pantry: nextPantry,
    statusBanner: `已提前开启【${season.name}】并获赠全套新声韵素材！`,
  };
}

export const MAX_PLOTS = 12;
export const PLOT_EXPANSION_COSTS = [25, 30, 40, 50, 60, 75] as const;

export function getNextPlotCost(currentPlotCount: number): number | null {
  if (currentPlotCount >= MAX_PLOTS) return null;
  const idx = Math.max(0, currentPlotCount - 6);
  return PLOT_EXPANSION_COSTS[idx] ?? PLOT_EXPANSION_COSTS[PLOT_EXPANSION_COSTS.length - 1];
}

export function expandGreenhousePlot(state: GameState): GameState {
  const cost = getNextPlotCost(state.plots.length);
  if (cost === null) {
    return {
      ...state,
      statusBanner: `温室花架已扩建至最大规模（${MAX_PLOTS} 个花盆位）！`,
    };
  }
  if (state.coins < cost) {
    return {
      ...state,
      statusBanner: `金币不足，扩建第 ${state.plots.length + 1} 号花盆需要 ${cost} 金币。`,
    };
  }

  const nextId = state.plots.length;
  const nextPlot: PlotState = {
    id: nextId,
    seedId: null,
    kind: null,
    growth: 0,
    wateredToday: false,
  };

  return {
    ...state,
    coins: state.coins - cost,
    plots: [...state.plots, nextPlot],
    statusBanner: `扩建成功！新增 ${nextId + 1} 号花盆（当前 ${nextId + 1}/${MAX_PLOTS} 盆），点击即可播种新拼音。`,
  };
}

export const SEED_PACK_COST = 6;
export const SEED_PACK_YIELD = 3;

export function buySeedPack(
  state: GameState,
  seedId: string,
  kind: SeedKind
): GameState {
  const unlocked =
    kind === 'initial'
      ? state.unlockedInitials.includes(seedId)
      : state.unlockedFinals.includes(seedId);
  if (!unlocked) return state;

  if (state.coins < SEED_PACK_COST) {
    return {
      ...state,
      statusBanner: `金币不足，购买「${seedId}」种子补给包需要 ${SEED_PACK_COST} 金币。`,
    };
  }

  return {
    ...state,
    coins: state.coins - SEED_PACK_COST,
    pantry: {
      ...state.pantry,
      [seedId]: (state.pantry[seedId] ?? 0) + SEED_PACK_YIELD,
    },
    statusBanner: `已购入「${seedId}」${kind === 'initial' ? '声母' : '韵母'}种子补给包（库存 +${SEED_PACK_YIELD}）！`,
  };
}

export function quickPlantToEmptyPlot(
  state: GameState,
  seedId: string,
  kind: SeedKind
): GameState {
  const emptyPlot = state.plots.find((p) => p.seedId === null);
  if (!emptyPlot) {
    return {
      ...state,
      statusBanner: '温室花架目前没有空花盆啦！可点击右上角「扩建」买新花盆，或清空旧花盆后再种。',
    };
  }
  return plantSeed(state, emptyPlot.id, seedId, kind);
}

export function selectAlchemyStage(
  state: GameState,
  stageId: AlchemyStageId
): GameState {
  const stage = getStageConfig(stageId);
  return {
    ...state,
    activeStageId: stageId,
    statusBanner:
      stageId === 'free'
        ? '已切换至【自由炼金】：自由组合库存声母、介母与韵母，炼制魔药交付村民委托。'
        : `已进入【${stage.title}】：${stage.subtitle}（试炼关卡免耗素材，通关奖励 +${stage.rewardGold} 金币）`,
  };
}

export function replayAlchemyStage(
  state: GameState,
  stageId: ChallengeStageId
): GameState {
  const stage = getStageConfig(stageId);
  return {
    ...state,
    activeStageId: stageId,
    stageProgress: { ...state.stageProgress, [stageId]: 0 },
    statusBanner: `重新开启【${stage.title}】试炼，从第 1 题开始挑战！`,
  };
}

export function getSeasonProgress(state: GameState): {
  current: (typeof SEASONS)[number];
  next: (typeof SEASONS)[number] | null;
  ordersToNext: number;
} {
  const current = SEASONS[state.seasonIndex] ?? SEASONS[0];
  const next = SEASONS[state.seasonIndex + 1] ?? null;
  const ordersToNext = next ? Math.max(0, next.requiredOrders - state.ordersCompleted) : 0;
  return { current, next, ordersToNext };
}
