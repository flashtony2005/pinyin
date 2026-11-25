import type { MedialLetter, ToneNumber } from '../data/pinyinResources';

export type ChallengeStageId = 'lvl1' | 'lvl2' | 'lvl3' | 'lvl4';
export type AlchemyStageId = 'free' | ChallengeStageId;

export interface StageTask {
  id: string;
  initial: string;
  medial: MedialLetter;
  final: string;
  tone: ToneNumber;
  /** Expected composed syllable key (e.g. 'ma', 'jia', 'ju', 'juan') */
  syllable: string;
  clue: string;
}

export interface AlchemyStageConfig {
  id: AlchemyStageId;
  shortLabel: string;
  title: string;
  stars: number; // 0 for free mode, 1..4 for challenge stages
  subtitle: string;
  allowMedial: boolean;
  blindMode: boolean;
  rewardGold: number;
  /** Letters guaranteed to be selectable in the alembic tray during this stage */
  featuredInitials: string[];
  featuredFinals: string[];
  tasks: StageTask[];
}

export const CHALLENGE_STAGE_IDS: ChallengeStageId[] = ['lvl1', 'lvl2', 'lvl3', 'lvl4'];

export const ALCHEMY_STAGES: AlchemyStageConfig[] = [
  {
    id: 'free',
    shortLabel: '自由',
    title: '自由炼金',
    stars: 0,
    subtitle: '自由组合已解锁声母、介母与韵母，随时接取村民委托',
    allowMedial: true,
    blindMode: false,
    rewardGold: 0,
    featuredInitials: [],
    featuredFinals: [],
    tasks: [],
  },
  {
    id: 'lvl1',
    shortLabel: '①两拼',
    title: '第 1 关 · 两拼入门',
    stars: 1,
    subtitle: '专注「声母 + 单韵母 + 四声」，暂隐介母槽降低干扰',
    allowMedial: false,
    blindMode: false,
    rewardGold: 30,
    featuredInitials: ['b', 'p', 'm', 'f', 'd', 't', 'n', 'l'],
    featuredFinals: ['a', 'o', 'e', 'i', 'u', 'ü'],
    tasks: [
      {
        id: 'lvl1_1',
        initial: 'm',
        medial: '',
        final: 'a',
        tone: 3,
        syllable: 'ma',
        clue: '声母 m + 韵母 a + 三声拐',
      },
      {
        id: 'lvl1_2',
        initial: 'b',
        medial: '',
        final: 'a',
        tone: 1,
        syllable: 'ba',
        clue: '声母 b + 韵母 a + 一声平',
      },
      {
        id: 'lvl1_3',
        initial: 'd',
        medial: '',
        final: 'i',
        tone: 4,
        syllable: 'di',
        clue: '声母 d + 韵母 i + 四声降',
      },
    ],
  },
  {
    id: 'lvl2',
    shortLabel: '②三拼',
    title: '第 2 关 · 三拼进阶',
    stars: 2,
    subtitle: '开启介母槽（i / u），掌握「声轻介快韵母响」',
    allowMedial: true,
    blindMode: false,
    rewardGold: 45,
    featuredInitials: ['j', 'q', 'x', 'd', 'g', 'h'],
    featuredFinals: ['a', 'o', 'i', 'u'],
    tasks: [
      {
        id: 'lvl2_1',
        initial: 'j',
        medial: 'i',
        final: 'a',
        tone: 1,
        syllable: 'jia',
        clue: '声母 j + 介母 i + 韵母 a · 一声平',
      },
      {
        id: 'lvl2_2',
        initial: 'd',
        medial: 'u',
        final: 'o',
        tone: 3,
        syllable: 'duo',
        clue: '声母 d + 介母 u + 韵母 o · 三声拐',
      },
      {
        id: 'lvl2_3',
        initial: 'h',
        medial: 'u',
        final: 'a',
        tone: 1,
        syllable: 'hua',
        clue: '声母 h + 介母 u + 韵母 a · 一声平',
      },
    ],
  },
  {
    id: 'lvl3',
    shortLabel: '③规则',
    title: '第 3 关 · 规则试炼',
    stars: 3,
    subtitle: '攻克 j/q/x 遇 ü 去两点、n/l 保留两点及三拼去点',
    allowMedial: true,
    blindMode: false,
    rewardGold: 60,
    featuredInitials: ['j', 'q', 'x', 'n', 'l'],
    featuredFinals: ['ü', 'an', 'a', 'i'],
    tasks: [
      {
        id: 'lvl3_1',
        initial: 'j',
        medial: '',
        final: 'ü',
        tone: 1,
        syllable: 'ju',
        clue: '去点魔法：j + 韵母 ü（写 ju）· 一声平',
      },
      {
        id: 'lvl3_2',
        initial: 'l',
        medial: '',
        final: 'ü',
        tone: 4,
        syllable: 'lü',
        clue: '保点规则：l + 韵母 ü（留两点）· 四声降',
      },
      {
        id: 'lvl3_3',
        initial: 'j',
        medial: 'ü',
        final: 'an',
        tone: 3,
        syllable: 'juan',
        clue: '三拼去点：j + 介母 ü + 韵母 an（写 juan）· 三声拐',
      },
    ],
  },
  {
    id: 'lvl4',
    shortLabel: '④听音',
    title: '第 4 关 · 听音大师',
    stars: 4,
    subtitle: '隐藏字母公式！听音辨别平翘舌、后鼻音与三拼音节',
    allowMedial: true,
    blindMode: true,
    rewardGold: 80,
    featuredInitials: ['j', 'q', 'sh', 'zh', 'l', 'h', 'b'],
    featuredFinals: ['a', 'u', 'i', 'an', 'ing'],
    tasks: [
      {
        id: 'lvl4_1',
        initial: 'sh',
        medial: '',
        final: 'u',
        tone: 1,
        syllable: 'shu',
        clue: '盲听 1/3：分辨翘舌声母与单韵母',
      },
      {
        id: 'lvl4_2',
        initial: 'q',
        medial: '',
        final: 'ing',
        tone: 1,
        syllable: 'qing',
        clue: '盲听 2/3：听清前声母与后鼻韵母',
      },
      {
        id: 'lvl4_3',
        initial: 'h',
        medial: 'u',
        final: 'a',
        tone: 4,
        syllable: 'hua',
        clue: '盲听终试：辨听三拼音节（声+介+韵）与四声',
      },
    ],
  },
];

export function getStageConfig(stageId: AlchemyStageId): AlchemyStageConfig {
  return ALCHEMY_STAGES.find((s) => s.id === stageId) ?? ALCHEMY_STAGES[0];
}

export function getNextChallengeStage(stageId: AlchemyStageId): ChallengeStageId | null {
  if (stageId === 'free') return 'lvl1';
  const idx = CHALLENGE_STAGE_IDS.indexOf(stageId);
  if (idx < 0 || idx >= CHALLENGE_STAGE_IDS.length - 1) return null;
  return CHALLENGE_STAGE_IDS[idx + 1];
}
